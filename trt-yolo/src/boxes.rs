use imageproc::rect::Rect;
use std::{fmt::Display, time::Instant};

use crate::coco;

#[derive(Clone, Copy, Debug)]
pub struct BoundingClass {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
    pub class: usize,
    pub confidence: f32,
    area_cache: f32,
}

impl BoundingClass {
    #[inline(always)]
    pub fn from_corners(x1: f32, y1: f32, x2: f32, y2: f32, class: usize, confidence: f32) -> Self {
        let x_min = x1.min(x2);
        let y_min = y1.min(y2);
        let x_max = x1.max(x2);
        let y_max = y1.max(y2);

        Self {
            x_min,
            y_min,
            x_max,
            y_max,
            class,
            confidence,
            area_cache: (x_max - x_min + 1.) * (y_max - y_min + 1.),
        }
    }

    #[inline(always)]
    pub fn from_center(
        cx: f32,
        cy: f32,
        width: f32,
        height: f32,
        class: usize,
        confidence: f32,
    ) -> Self {
        let w2 = width / 2.;
        let h2 = height / 2.;

        Self {
            x_min: cx - w2,
            y_min: cy - h2,
            x_max: cx + w2,
            y_max: cy + h2,
            class,
            confidence,
            area_cache: (width + 1.) * (height + 1.),
        }
    }

    #[inline(always)]
    pub fn xmin(&self) -> f32 {
        self.x_min
    }

    #[inline(always)]
    pub fn ymin(&self) -> f32 {
        self.y_min
    }

    #[inline(always)]
    pub fn xmax(&self) -> f32 {
        self.x_max
    }

    #[inline(always)]
    pub fn ymax(&self) -> f32 {
        self.y_max
    }

    #[inline(always)]
    pub fn corners(&self) -> ((f32, f32), (f32, f32)) {
        ((self.x_min, self.y_min), (self.x_max, self.y_max))
    }

    #[inline(always)]
    pub fn width(&self) -> f32 {
        self.x_max - self.x_min
    }

    #[inline(always)]
    pub fn height(&self) -> f32 {
        self.y_max - self.y_min
    }

    #[inline(always)]
    pub fn area(&self) -> f32 {
        self.area_cache
    }

    #[inline(always)]
    pub fn class_name(&self) -> &'static str {
        coco::NAMES[self.class]
    }

    #[inline]
    pub fn conf_rgb(&self) -> [u8; 3] {
        let c = self.confidence * 2. - 1.;
        if c >= 0. {
            [((1. - c.clamp(0., 1.)) * 255.) as _, 255, 0]
        } else {
            [255, ((1. + c.clamp(-1., 0.)) * 255.) as _, 0]
        }
    }

    pub fn rescale(
        &self,
        old_width: f32,
        old_height: f32,
        new_width: f32,
        new_height: f32,
    ) -> Self {
        let sx = new_width / old_width;
        let sy = new_height / old_height;

        let ((xmin, ymin), (xmax, ymax)) = self.corners();
        let xmin = xmin.clamp(0., old_width - 1.) * sx;
        let ymin = ymin.clamp(0., old_height - 1.) * sy;
        let xmax = xmax.clamp(0., old_width - 1.) * sx;
        let ymax = ymax.clamp(0., old_height - 1.) * sy;
        Self::from_corners(xmin, ymin, xmax, ymax, self.class, self.confidence)
    }

    #[inline(always)]
    pub fn to_imageproc_rect(&self) -> Rect {
        Rect::at(self.xmin() as _, self.ymin() as _).of_size(self.width() as _, self.height() as _)
    }

    #[inline(always)]
    pub fn iou(&self, other: &BoundingClass) -> f32 {
        let i_xmin = self.xmin().max(other.xmin());
        let i_ymin = self.ymin().max(other.ymin());

        let i_xmax = self.xmax().min(other.xmax());
        let i_ymax = self.ymax().min(other.ymax());

        let i_area = (i_xmax - i_xmin + 1.).max(0.) * (i_ymax - i_ymin + 1.).max(0.);
        i_area / (self.area() + other.area() - i_area)
    }
}

impl Display for BoundingClass {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} ({:.1}%) at ({:.0}, {:.0}) to ({:.0}, {:.0}) ",
            coco::NAMES[self.class],
            self.confidence * 100.,
            self.xmin(),
            self.ymin(),
            self.xmax(),
            self.ymax()
        )
    }
}

/// Non-maximum suppression (NMS) filters overlapping bounding boxes that have an intersection-over-
/// union (IoU) greater or equal than the specified `iou_threshold` with previously selected boxes.
///
/// Boxes are filtered based on `score_threshold` and ranked based on their score. As such, lower
/// scoring boxes are removed when overlapping with another (higher scoring) box.
pub fn nms_cpu(
    outputs: &[half::f16],
    shape: [usize; 3],
    iou_threshold: f32,
    score_threshold: f32,
) -> Vec<BoundingClass> {
    if shape[0] != 1 {
        panic!("unexpected tensor shape for nms {:?}", shape);
    }

    let start_time = Instant::now();

    let mut filtered_boxes = Vec::new();
    for bbox_off in 0..shape[2] {
        let mut field_off = bbox_off;
        let cx = outputs[field_off];
        field_off += shape[2];

        let cy = outputs[field_off];
        field_off += shape[2];

        let width = outputs[field_off];
        field_off += shape[2];

        let height = outputs[field_off];
        field_off += shape[2];

        let mut best_class = -1;
        let mut best_score = score_threshold;
        for i in 0..(shape[1] - 4) {
            let score = outputs[field_off].to_f32();
            if score > best_score {
                best_class = i as i64;
                best_score = score;
            }

            field_off += shape[2];
        }

        if best_class < 0 {
            continue;
        }

        let b = BoundingClass::from_center(
            cx.to_f32(),
            cy.to_f32(),
            width.to_f32(),
            height.to_f32(),
            best_class as _,
            best_score,
        );

        filtered_boxes.push(b);
    }

    filtered_boxes.sort_unstable_by(|a, b| b.confidence.total_cmp(&a.confidence));

    println!(
        "class scoring took {:?}us",
        start_time.elapsed().as_micros()
    );

    let mut acc = Vec::new();

    let start_time = Instant::now();
    for b in filtered_boxes {
        let any_iou = acc.iter().any(|other| b.iou(other) > iou_threshold);

        if !any_iou {
            acc.push(b);
        }
    }

    println!("accing took {:?}us", start_time.elapsed().as_micros());

    acc
}
