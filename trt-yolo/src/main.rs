use std::time::Instant;

use image::{imageops, EncodableLayout, GenericImageView, Rgb};
use tensorrt::RuntimeEngineContext;
use trt_yolo::{boxes::nms_cpu, Inferer, Which};

fn main() {
    let which = Which::best();

    let img = image::open("../detect-samples/cars.jpg")
        .unwrap()
        .into_rgb8();
    let crop_size = img.width().min(img.height());
    let left = (img.width() - crop_size) / 2;

    let img = img.view(left, 0, crop_size, crop_size);
    let mut img = imageops::resize(&*img, 640, 640, imageops::FilterType::Nearest);

    let runtime = RuntimeEngineContext::new_engine_slice(&which.plan_data().unwrap());

    let in_elms = which.input_elems();
    let out_elms = which.out_elems();
    let mut inferer = Inferer::from_exec_ctx(runtime.as_ctx(), in_elms, out_elms);

    let mut out_buf = vec![half::f16::from_f32_const(0.); out_elms].into_boxed_slice();
    for _ in 0..3 {
        let start_time = Instant::now();
        inferer.run(img.as_bytes(), &mut out_buf);

        println!("infer took {}us", start_time.elapsed().as_micros());
    }

    nms_cpu(&out_buf, which.out_shape(), 0.65, 0.5)
        .into_iter()
        .for_each(|b| {
            println!("{b}");
            let bound_rect = b.to_imageproc_rect();
            imageproc::drawing::draw_hollow_rect_mut(&mut img, bound_rect, Rgb(b.conf_rgb()));
        });

    img.save("out.png").unwrap();
}

// fn img_to_channels(img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, out: &mut [half::f16]) {
//     let start_time = Instant::now();
//     ndarray::Array::from_shape_vec(
//         [1, img.height() as usize, img.width() as usize, 3],
//         img.into_vec(),
//     )
//     .unwrap()
//     .permuted_axes([0, 3, 1, 2])
//     .into_iter()
//     .zip(out)
//     .for_each(|(p, v)| {
//         *v = half::f16::from(p) / half::f16::from_f32_const(255.);
//     });

//     println!("stride_conv took {}us", start_time.elapsed().as_micros());
// }
