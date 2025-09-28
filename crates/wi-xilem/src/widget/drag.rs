use masonry::core::{PointerInfo, PointerState};
use xilem::dpi::PhysicalPosition;

#[derive(Debug)]
struct Drag<T> {
    pointer: PointerInfo,
    start_pos: PhysicalPosition<f64>,
    start_info: T,
}

#[derive(Debug)]
pub struct DragHandler<T>(Option<Drag<T>>);

impl<T> DragHandler<T> {
    #[inline]
    pub fn new() -> Self { Self(None) }

    pub fn in_drag(&self) -> bool { self.0.is_some() }

    pub fn begin_drag(&mut self, pointer: PointerInfo, state: &PointerState, start_info: T) {
        self.0 = Some(Drag {
            pointer,
            start_pos: state.position,
            start_info,
        });
    }

    pub fn update_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        update: impl FnOnce(&T, PhysicalPosition<f64>, PhysicalPosition<f64>),
    ) -> bool {
        let Some(drag) = self.0.as_mut() else {
            return false;
        };

        if *pointer != drag.pointer {
            return false;
        }

        update(&drag.start_info, drag.start_pos, state.position);
        true
    }

    pub fn complete_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        update: impl FnOnce(&T, PhysicalPosition<f64>, PhysicalPosition<f64>),
    ) {
        if self.update_drag(pointer, state, update) {
            self.0 = None;
        }
    }

    pub fn cancel_drag(&mut self, pointer: Option<&PointerInfo>, reset: impl FnOnce(&T)) {
        let Some(drag) = self.0.as_mut() else {
            return;
        };

        if let Some(p) = pointer
            && *p != drag.pointer
        {
            return;
        }

        reset(&drag.start_info);
        self.0 = None;
    }
}
