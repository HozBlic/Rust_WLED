// #![windows_subsystem = "windows"]

use display_info::DisplayInfo;
use image::{imageops, imageops::resize, imageops::FilterType, RgbaImage};
use serialport::SerialPortType;
use std::io::{self, Write};
use std::process;
use std::time::Duration;
use std::time::Instant;
use win_desktop_duplication::*;
use win_desktop_duplication::{devices::*, tex_reader::*};
use colorsys::{Rgb, ColorTransform, SaturationInSpace, Hsl};

fn main() {
    let ports = serialport::available_ports().expect("No ports found!");
    let mut com_port = String::new();

    for port in ports {
        // println!("{:?}", port.port_type);

        if let SerialPortType::UsbPort(usb_info) = &port.port_type {
            let product = usb_info.product.as_deref().unwrap_or("Unknown");
            // println!("{}", product);

            if product.contains("USB-SERIAL CH340") {
                let start_bytes = product.find("(").unwrap_or(0) + 1;
                let end_bytes = product.find(")").unwrap_or(product.len());
                com_port = product[start_bytes..end_bytes].to_string();
            }
        }
    }
    println!("{}", com_port);

    let display_infos = DisplayInfo::all().unwrap();
    let monitor_index = 1;

    let width;
    let height;

    let width_pixel = 42;
    let height_pixel = 23;

    if let Some(_value) = display_infos.get(monitor_index) {
        width = display_infos[monitor_index].width;
        height = display_infos[monitor_index].height;
    } else {
        process::exit(1);
    }

    // this is required to be able to use desktop duplication api
    set_process_dpi_awareness();
    co_init();

    // select gpu and output you want to use.
    let adapter = AdapterFactory::new().get_adapter_by_idx(0).unwrap();
    let output = adapter
        .get_display_by_idx(monitor_index.try_into().unwrap())
        .unwrap();

    // get output duplication api
    let mut dupl = DesktopDuplicationApi::new(adapter, output.clone()).unwrap();

    // Optional: get TextureReader to read GPU textures into CPU.
    let (device, ctx) = dupl.get_device_and_ctx();
    let mut texture_reader = TextureReader::new(device, ctx);

    // create a vector to hold picture data;
    let mut pic_data: Vec<u8> = vec![0; 0];

    let now = Instant::now();
    let mut i = 1;

    loop {
        // this api send one frame per vsync. the frame also has cursor pre drawn
        output.wait_for_vsync().unwrap();
        let tex = dupl.acquire_next_frame_now();

        if let Ok(tex) = tex {
            texture_reader.get_data(&mut pic_data, &tex);

            // Convert the raw data to an RgbaImage
            let mut image = RgbaImage::from_raw(width, height, pic_data.clone())
                .expect("Failed to create image from raw data");

            let sub_image = imageops::crop(&mut image, 0, 0, width, 60);
            let crop_image = sub_image.to_image();
            let top_image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                resize(&crop_image, width_pixel, 1, FilterType::Nearest);

            let sub_image = imageops::crop(&mut image, 0, height - 60, width, 60);
            let crop_image = sub_image.to_image();
            let bottom_image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                resize(&crop_image, width_pixel, 1, FilterType::Nearest);

            let sub_image = imageops::crop(&mut image, 0, 60, 60, height - 120);
            let crop_image = sub_image.to_image();
            let left_image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                resize(&crop_image, 1, height_pixel, FilterType::Nearest);

            let sub_image = imageops::crop(&mut image, width - 60, 60, 60, height - 120);
            let crop_image = sub_image.to_image();
            let right_image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                resize(&crop_image, 1, height_pixel, FilterType::Nearest);

            // image
            //     .save("output.png")
            //     .expect("Failed to save resized image");
            // bottom_image
            //     .save("output1.png")
            //     .expect("Failed to save resized image");
            // left_image
            //     .save("output2.png")
            //     .expect("Failed to save resized image");
            // right_image
            //     .save("output3.png")
            //     .expect("Failed to save resized image");

            // First and last row
            let mut first_row: Vec<_> = top_image.pixels().collect();
            let mut last_row: Vec<_> = bottom_image.pixels().collect();
            let mut left_column: Vec<_> = left_image.pixels().collect();
            let mut right_column: Vec<_> = right_image.pixels().collect();

            // Convert to hexadecimal format
            // let rgba_to_hex = |pixel: &image::Rgba<u8>| -> String {
            //     format!("{:02X}{:02X}{:02X}", pixel[2], pixel[1], pixel[0])
            // };

            /******* */


            first_row.reverse();
            right_column.reverse();


            first_row.extend(left_column);
            first_row.extend(last_row);
            first_row.extend(right_column);


            let send_hex: Vec<String> =
            first_row
                .iter()
                .map(|pixel| {
                    let mut rgb_pixel = Rgb::from((pixel[2], pixel[1], pixel[0]));
                    let mut hsl_pixel = Hsl::from(rgb_pixel);
    
                    if hsl_pixel.lightness() > 5.0 && hsl_pixel.lightness() < 30.0 {
                        hsl_pixel.set_lightness(30.0);
                    }
    
                    if hsl_pixel.saturation() > 5.0 && hsl_pixel.saturation() < 60.0 {
                        hsl_pixel.set_saturation(60.0);
                    }
    
                    rgb_pixel = Rgb::from(hsl_pixel);
                    
                    let mut rgb_hex: String = rgb_pixel.to_hex_string();
                    rgb_hex.remove(0);
    
                    return rgb_hex;
                })
                .collect();

            let result = send_hex.join("") + "\n";

            if let Err(e) = send_to_serial_port(&com_port, &result) {
                eprintln!("Failed to send data to serial port: {}", e);
            }

            let elapsed = now.elapsed();
            let elapsed_secs = elapsed.as_secs_f64();

            let fps = i as f64 / elapsed_secs;
            println!("{:.2} FPS", fps);

            i = i + 1;

            // print!("{}", result);
        }
    }
}

fn send_to_serial_port(port_name: &str, data: &str) -> io::Result<()> {
    // Open the serial port
    let mut port = serialport::new(port_name, 1000000)
        .timeout(Duration::from_millis(1000))
        .open()?;

    // Write data to the serial port
    port.write_all(data.as_bytes())?;
    print!("{}", data);

    Ok(())
}
