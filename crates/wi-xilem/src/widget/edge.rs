use std::f64::consts::{PI, TAU};

use masonry::kurbo::{
    Arc, ArcAppendIter, ParamCurveArclen, ParamCurveArea, PathEl, Point, Rect, Shape, Vec2,
};

enum EdgeIterState {
    Arc1Start(f64, Arc, Arc),
    Arc1Append(f64, ArcAppendIter, Arc),
    Arc2Start(f64, Arc),
    Arc2Append(ArcAppendIter),
}

pub struct EdgeIter(EdgeIterState);

impl Iterator for EdgeIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<Self::Item> {
        // Taken from kurbo::arc
        #[inline]
        fn arc_start(arc: &Arc) -> Vec2 {
            let (angle_sin, angle_cos) = arc.start_angle.sin_cos();
            let u = arc.radii.x * angle_cos;
            let v = arc.radii.y * angle_sin;
            arc.center.to_vec2() + rotate_pt(Vec2::new(u, v), arc.x_rotation)
        }

        // Taken from kurbo::arc
        fn rotate_pt(pt: Vec2, angle: f64) -> Vec2 {
            let (angle_sin, angle_cos) = angle.sin_cos();
            Vec2::new(
                pt.x * angle_cos - pt.y * angle_sin,
                pt.x * angle_sin + pt.y * angle_cos,
            )
        }

        loop {
            break Some(match &mut self.0 {
                &mut EdgeIterState::Arc1Start(t, a, b) => {
                    self.0 = EdgeIterState::Arc1Append(t, a.append_iter(t), b);
                    PathEl::MoveTo(arc_start(&a).to_point())
                },
                &mut EdgeIterState::Arc1Append(t, ref mut a, b) => {
                    let Some(e) = a.next() else {
                        self.0 = EdgeIterState::Arc2Start(t, b);
                        continue;
                    };
                    e
                },
                &mut EdgeIterState::Arc2Start(t, a) => {
                    self.0 = EdgeIterState::Arc2Append(a.append_iter(t));
                    PathEl::LineTo(arc_start(&a).to_point())
                },
                EdgeIterState::Arc2Append(a) => a.next()?,
            });
        }
    }
}

pub struct Edge {
    from: Point,
    to: Point,
    bias_upward: bool,
}

impl Edge {
    #[inline]
    pub fn new(from: Point, to: Point, bias_upward: bool) -> Self {
        Self {
            from,
            to,
            bias_upward,
        }
    }

    #[inline]
    fn arcs_biased(&self, radius: f64) -> (Arc, Arc) {
        const ANGLE_OFFS: f64 = PI * -0.5;

        let Self {
            from,
            to,
            bias_upward,
        } = *self;
        let radii = Vec2::new(radius, radius);

        let sign = 1.0 - 2.0 * f64::from(bias_upward);

        let from_ctr = from + Vec2::new(0.0, radius * sign);
        let to_ctr = to + Vec2::new(0.0, radius * sign);

        let angle = (to.y - from.y).atan2(to.x - from.x);
        let from_sweep =
            angle + TAU * sign * f64::from(angle.is_sign_negative() ^ sign.is_sign_negative());

        let to_sweep = TAU * sign - from_sweep;

        (
            Arc {
                center: from_ctr,
                radii,
                start_angle: ANGLE_OFFS * sign,
                sweep_angle: from_sweep,
                x_rotation: 0.0,
            },
            Arc {
                center: to_ctr,
                radii,
                start_angle: ANGLE_OFFS * sign + from_sweep,
                sweep_angle: to_sweep,
                x_rotation: 0.0,
            },
        )
    }

    #[inline]
    fn arcs_scurve(&self, radius: f64) -> (Arc, Arc) {
        const ANGLE_OFFS: f64 = PI * -0.5;

        let Self {
            from,
            to,
            bias_upward: _,
        } = *self;
        let radii = Vec2::new(radius, radius);

        let sign = (to.y - from.y).signum();

        let from_ctr = from + Vec2::new(0.0, radius * sign);
        let to_ctr = to + Vec2::new(0.0, radius * -sign);

        let hyp = (to_ctr - from_ctr).length();
        let hyp_angle = (to_ctr.y - from_ctr.y).atan2(to_ctr.x - from_ctr.x);

        let sweep = hyp_angle + (2.0 * radius / hyp.max(1e-7)).asin() * sign;

        (
            Arc {
                center: from_ctr,
                radii,
                start_angle: ANGLE_OFFS * sign,
                sweep_angle: sweep,
                x_rotation: 0.0,
            },
            Arc {
                center: to_ctr,
                radii,
                start_angle: ANGLE_OFFS * -sign + sweep,
                sweep_angle: -sweep,
                x_rotation: 0.0,
            },
        )
    }

    pub fn arcs(&self) -> (Arc, Arc) {
        const RADIUS: f64 = 24.0;

        let Self { from, to, .. } = *self;

        let radius = {
            let delta = to - from;

            (delta.x.abs() / 2.0)
                .max(delta.y.abs() / 4.0)
                .clamp(0.0, RADIUS)
        };

        if to.x < from.x && (to.y - from.y).abs() < 3.0 * radius {
            self.arcs_biased(radius)
        } else {
            self.arcs_scurve(radius)
        }
    }
}

impl Shape for Edge {
    type PathElementsIter<'iter> = EdgeIter;

    fn path_elements(&self, tolerance: f64) -> Self::PathElementsIter<'_> {
        let (a, b) = self.arcs();
        EdgeIter(EdgeIterState::Arc1Start(tolerance, a, b))
    }

    fn area(&self) -> f64 { self.path_segments(0.1).map(|seg| seg.signed_area()).sum() }

    fn perimeter(&self, accuracy: f64) -> f64 {
        self.path_segments(0.1)
            .map(|seg| seg.arclen(accuracy))
            .sum()
    }

    fn winding(&self, pt: Point) -> i32 { self.path_segments(0.1).map(|seg| seg.winding(pt)).sum() }

    fn bounding_box(&self) -> Rect {
        let (a, b) = self.arcs();
        a.bounding_box().union(b.bounding_box())
    }
}
