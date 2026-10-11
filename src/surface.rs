//! Transparent, click-through layer-shell strips along the screen edges.
//! The top strip carries the sweep and is redrawn on every frame callback;
//! the others only draw while a state puts something on them.

use std::time::Instant;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData, Region},
    output::{OutputHandler, OutputState},
    reexports::client::{
        globals::registry_queue_init,
        protocol::{wl_output, wl_shm, wl_surface},
        Connection, QueueHandle,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use tiny_skia::{Color, Pixmap};

use crate::edge::Edge;
use crate::machine::Machine;
use crate::pill::Pill;

/// Layer namespace, for compositor rules (e.g. niri `layer-rule`).
const NAMESPACE: &str = "random-fx";

pub fn run(
    machine: Machine,
    thickness: u32,
    pill: Option<Pill>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init(&conn)?;
    let qh = queue.handle();

    let compositor = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;
    // An empty input region lets every click fall through to what's below.
    let region = Region::new(&compositor)?;
    let layer = |anchor: Anchor, (w, h): (u32, u32)| {
        let surface = compositor.create_surface(&qh);
        surface.set_input_region(Some(region.wl_region()));
        let layer =
            layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some(NAMESPACE), None);
        layer.set_anchor(anchor);
        layer.set_size(w, h);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer
    };

    let strips = Edge::ALL.map(|edge| {
        let (anchor, size) = match edge {
            Edge::Top => (Anchor::TOP | Anchor::LEFT | Anchor::RIGHT, (0, thickness)),
            Edge::Bottom => (
                Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
                (0, thickness),
            ),
            Edge::Left => (Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM, (thickness, 0)),
            Edge::Right => (Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM, (thickness, 0)),
        };
        (edge, Surface::new(layer(anchor, size)))
    });
    // Positioned from the screen's top-left corner, over the clock pill.
    let pill = pill.map(|pill| {
        let layer = layer(Anchor::TOP | Anchor::LEFT, pill.surface_size());
        let (x, y) = pill.surface_origin();
        layer.set_margin(y, 0, 0, x);
        (pill, Surface::new(layer))
    });

    let mut app = App {
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        pool: SlotPool::new(4096 * thickness as usize * 4, &shm)?,
        shm,
        strips,
        pill,
        thickness: thickness as f32,
        machine,
        start: Instant::now(),
        started: false,
        exit: false,
    };

    while !app.exit {
        queue.blocking_dispatch(&mut app)?;
    }
    Ok(())
}

/// One layer surface and what we last drew on it.
struct Surface {
    layer: LayerSurface,
    /// `None` until the first configure tells us our size.
    pixmap: Option<Pixmap>,
    /// Whether the last buffer we committed had anything on it.
    showing: bool,
}

impl Surface {
    fn new(layer: LayerSurface) -> Self {
        // The initial commit, without a buffer, asks for a configure.
        layer.commit();
        Self {
            layer,
            pixmap: None,
            showing: false,
        }
    }

    fn resize(&mut self, (w, h): (u32, u32)) {
        if self.pixmap.as_ref().map(|p| (p.width(), p.height())) != Some((w, h)) {
            self.pixmap = Pixmap::new(w.max(1), h.max(1));
            // Redraw at the new size even when nothing is on it.
            self.showing = true;
        }
    }

    /// Paints and commits a frame. Surfaces that are and were empty are
    /// skipped (they keep their last, empty, buffer and cost nothing),
    /// unless `drive` is set: then the frame is always drawn and requests
    /// the frame callback that drives the whole animation.
    fn present(
        &mut self,
        pool: &mut SlotPool,
        qh: &QueueHandle<App>,
        has_content: bool,
        drive: bool,
        paint: impl FnOnce(&mut Pixmap),
    ) {
        let Some(pixmap) = self.pixmap.as_mut() else {
            return;
        };
        if !drive && !has_content && !self.showing {
            return;
        }
        pixmap.fill(Color::TRANSPARENT);
        paint(pixmap);
        self.showing = has_content;

        let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
        let (buffer, canvas) = pool
            .create_buffer(w, h, w * 4, wl_shm::Format::Argb8888)
            .expect("create shm buffer");
        // tiny-skia is premultiplied RGBA; ARGB8888 is premultiplied BGRA in memory.
        let (dst, _) = canvas.as_chunks_mut::<4>();
        let (src, _) = pixmap.data().as_chunks::<4>();
        for (d, &[r, g, b, a]) in dst.iter_mut().zip(src) {
            *d = [b, g, r, a];
        }

        let surface = self.layer.wl_surface();
        surface.damage_buffer(0, 0, w, h);
        if drive {
            surface.frame(qh, FrameCallbackData(surface.clone()));
        }
        buffer.attach_to(surface).expect("attach buffer");
        self.layer.commit();
    }
}

struct App {
    registry: RegistryState,
    outputs: OutputState,
    shm: Shm,
    pool: SlotPool,
    /// Indexed by `Edge::index`.
    strips: [(Edge, Surface); 4],
    pill: Option<(Pill, Surface)>,
    thickness: f32,
    machine: Machine,
    start: Instant,
    started: bool,
    exit: bool,
}

impl App {
    /// Screen size, as learned from the strips that span it.
    fn screen(&self) -> (f32, f32) {
        let len = |e: Edge| {
            self.strips[e.index()].1.pixmap.as_ref().map_or(0.0, |p| {
                (if e.is_horizontal() {
                    p.width()
                } else {
                    p.height()
                }) as f32
            })
        };
        (len(Edge::Top), len(Edge::Left))
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) {
        let screen = self.screen();
        let frame = self
            .machine
            .frame(self.start.elapsed().as_secs_f32(), screen);

        for (edge, surface) in &mut self.strips {
            let edge = *edge;
            let has_lines = frame.lines().any(|(e, l)| e == edge && l.is_visible());
            let transform = edge.transform(self.thickness);
            // The top strip's frame callbacks drive the whole animation.
            surface.present(&mut self.pool, qh, has_lines, edge == Edge::Top, |pixmap| {
                for (_, line) in frame.lines().filter(|(e, _)| *e == edge) {
                    line.paint(pixmap, edge.length(screen), transform);
                }
            });
        }
        if let Some((pill, surface)) = &mut self.pill {
            let marks = frame.pill;
            surface.present(&mut self.pool, qh, !marks.is_empty(), false, |pixmap| {
                pill.paint(pixmap, marks)
            });
        }
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let is = |s: &Surface| s.layer.wl_surface() == layer.wl_surface();
        let strip = self.strips.iter_mut().find(|(_, s)| is(s));
        let is_top = strip.as_ref().is_some_and(|(e, _)| *e == Edge::Top);
        let surface = match strip {
            Some((_, s)) => s,
            None => match &mut self.pill {
                Some((_, s)) if is(s) => s,
                _ => return,
            },
        };
        surface.resize(configure.new_size);
        // Later frames are driven by the top strip's frame callbacks.
        if !self.started && is_top {
            self.started = true;
            self.draw(qh);
        }
    }
}

impl CompositorHandler for App {
    fn frame(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        self.draw(qh);
    }

    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState];
}

smithay_client_toolkit::delegate_registry!(App);
smithay_client_toolkit::delegate_dispatch2!(App);
