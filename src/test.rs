use super::mock_string;
use super::overlay_bonk;
use super::ImageData;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use image::imageops;
#[test]
fn test_mock() {
    let test_string = "mock this bitch";
    let mocked_string = mock_string(test_string);
    println!("{}", mocked_string);
    assert_eq!(test_string.len(), mocked_string.len());
}

#[test]
fn test_bonk() {
    /*
    let token = match env::var("DISCORD_TOKEN") {
        Ok(tok) => tok,
        Err(_) => {
            let mut content = String::new();
            File::open("assets/key").expect("could not find env DISCORD_TOKEN or file containing bot key")
                .read_to_string(&mut content).expect("could not read contents of assets/key");
            content
        },
    };

    let mut client = Client::new(&token, Handler).expect("Error creating client");
    client.start().unwrap();
    */
    let test_avatar = image::io::Reader::open("assets/test/test.png").unwrap()
        .decode().unwrap();
    let meta_data: Vec<ImageData> = serde_json::from_str(&std::fs::read_to_string("assets/bonk_locations.json").unwrap()).unwrap();
    for meta in meta_data {
        let bonked_avatar = overlay_bonk(test_avatar.clone(), &meta).unwrap();
        bonked_avatar.save_with_format(format!("assets/test/{}", meta.name), image::ImageFormat::Png).unwrap();
    }
}