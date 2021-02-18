use image::imageops;
use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageData {
    name: String,
    bonkee_x: u32,
    bonkee_y: u32,
    bonkee_width: u32,
    bonkee_height: u32,
    bonk_label_x: u32,
    bonk_label_y: u32,
    bonk_label_width: u32,
    bonk_label_height: u32,
    bonkee_top: bool
}
pub fn overlay_bonk(avatar: image::DynamicImage, meta: &ImageData) -> Result<image::DynamicImage, String> {
    let bonk_image = match image::open(format!("./assets/{}", meta.name)) {
        Ok(image) => image,
        Err(e) => return Err(format!("could not open bonk image: {}", e)),
    };
    let resized_avatar = imageops::resize(&avatar, meta.bonkee_width, meta.bonkee_height, imageops::FilterType::Nearest);
    let bonk_label = match image::open("./assets/bonklabel.png") {
        Ok(image) => image,
        Err(e) => return Err(format!("could not open bonk image: {}", e)),
    };
    let resized_label = imageops::resize(&bonk_label, meta.bonk_label_width, meta.bonk_label_height, imageops::FilterType::Nearest);
    let mut bonk_image_copy = bonk_image.clone();
    //to get actual coordinates, subtract half of width from x and half of height from y
    imageops::overlay(&mut bonk_image_copy, &resized_avatar, meta.bonkee_x - meta.bonkee_width/2, meta.bonkee_y - meta.bonkee_height/2);
    if !meta.bonkee_top {
        imageops::overlay(&mut bonk_image_copy, &bonk_image, 0, 0);
    }
    imageops::overlay(&mut bonk_image_copy, &resized_label, meta.bonk_label_x - meta.bonk_label_width/2, meta.bonk_label_y - meta.bonk_label_height/2);
    Ok(bonk_image_copy)
}

pub fn choose_bonk() -> Result<ImageData, String> {
    let meta_data: Vec<ImageData> = match std::fs::read_to_string("./assets/bonk_locations.json") {
        Ok(string) => match serde_json::from_str(&string) {
            Ok(data) => data,
            Err(e) => return Err(format!("could not parse JSON: {}", e)),
        },
        Err(e) => return Err(format!("could not read 'bonk_locations.json': {}", e)),
    };
    let meta = match meta_data.choose(&mut rand::thread_rng()) {
        Some(item) => item,
        None => return Err(format!("meta data has no elements to choose from")),
    };
    Ok(meta.clone())
}