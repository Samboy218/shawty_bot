use std::env;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::time::{Duration, Instant};
use std::error::Error;
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use serenity:: {
    async_trait,
    model::{channel::Message, channel::ReactionType, gateway::Ready, gateway::Activity},
    prelude::*,
    framework::StandardFramework,
    framework::standard::{
        CommandResult, macros::{group, command},
    },
    utils::{MessageBuilder},
};

#[cfg(test)]
mod test;

mod scheduler;
mod bonker;

struct MockTracker;

impl TypeMapKey for MockTracker {
    type Value = HashMap<u64, isize>;
}

struct BotOwner;

impl TypeMapKey for BotOwner {
    type Value = u64;
}

struct StatusTimer;
impl TypeMapKey for StatusTimer{
    type Value = Instant;
}

struct ReminderList;
impl TypeMapKey for ReminderList{
    type Value = Vec<Reminder>;
}



#[derive(Serialize, Deserialize, Debug, Clone)]
struct Reminder {
    date_time: chrono::NaiveDateTime,
    message: serenity::model::channel::Message,
    verification_message: serenity::model::channel::Message,
}

#[tokio::main]
async fn main() {
    println!("{}", mock_string("this is a test string"));

    let token = match env::var("DISCORD_TOKEN") {
        Ok(tok) => tok,
        Err(_) => {
            let mut content = String::new();
            File::open("./assets/key").expect("could not find env DISCORD_TOKEN or file containing bot key")
                .read_to_string(&mut content).expect("could not read contents of assets/key");
            content
        },
    };
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&MOCKER_GROUP);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<MockTracker>(HashMap::default());
        data.insert::<BotOwner>(277158017869414400);
        data.insert::<StatusTimer>(Instant::now());
        //attempt to load the reminder list from assets/reminder_list.json
        let mut reminder_list: Vec<Reminder> = match std::fs::read_to_string("./assets/reminder_list.json") {
            Ok(string) => match serde_json::from_str(&string) {
                Ok(data) => data,
                Err(e) => {
                    println!("could not parse JSON: {}", e);
                    Vec::new()
                },
            },
            Err(e) => {
                println!("could not read 'reminder_list.json': {}", e);
                Vec::new()
            },
        };
        let now = chrono::Local::now().naive_local();
        let num_read = reminder_list.len();
        reminder_list.retain(|reminder| reminder.date_time > now);
        if num_read > reminder_list.len() {
            println!("purged {} expired timers that should have been fired", num_read-reminder_list.len());
        }
        data.insert::<ReminderList>(reminder_list);
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

#[group("mocker")]
#[commands(mock, help, bonk, remind, flip)]
struct Mocker;

#[command]
async fn mock(ctx: &Context, msg: &Message) -> CommandResult {
    //add all mentioned users to the mock tracker
    let me = match ctx.http.as_ref().get_current_user().await {
        Ok(user) => *user.id.as_u64(),
        Err(e) => {
            println!("couldn't get current user??? {}", e);
            0
        }
    };
    let bot_owner = {
        let data = ctx.data.read().await;
        *data.get::<BotOwner>().expect("could not get BotOwner!")
    };
    for mentioned in &msg.mentions {
        let id = *mentioned.id.as_u64();
        if id != me && id != bot_owner {
            println!("now tracking user: {}", mentioned.name);
            track_mocker(ctx, id, 3).await;
        }
    }
    Ok(())
}

#[command]
async fn flip(ctx: &Context, msg: &Message) -> CommandResult {
    let myval ={
        let mut rng = rand::thread_rng();
        match rng.gen_bool(0.5) {
            true => "Heads",
            false => "Tails",
        }
    };
    if let Err(why) = msg.channel_id.say(ctx, format!("{}", myval)).await {
        println!("Could not send message: {}", why);
    }
    Ok(())
}

#[command]
async fn bonk(ctx: &Context, msg: &Message) -> CommandResult {
    let bonkee = match msg.mentions.get(0) {
        Some(user) => user,
        None => &msg.author,
    };
    let image_url = match bonkee.avatar_url() {
        Some(url) => str::replace(&url, ".webp", ".png"),
        None => {
            println!("no image url");
            return Ok(())
        }
    };
    let avatar = match reqwest::blocking::get(&image_url) {
        Ok(resp) => match resp.bytes() {
            Ok(text) => match image::load_from_memory_with_format(&text, image::ImageFormat::Png) {
                Ok(image) => image,
                Err(e) => {
                    println!("could not parse avatar image: {}", e);
                    return Ok(())
                }
            },
            Err(e) => {
                println!("could not get image response body: {}", e);
                return Ok(())
            } 
        },
        Err(e) => {
            println!("could not fetch image: {}", e);
            return Ok(())
        }
    };
    let bonk_choice = match bonker::choose_bonk() {
        Ok(choice) => choice,
        Err(e) => {
            println!("{}", e);
            return Ok(())
        }
    };
    let bonk_image = match bonker::overlay_bonk(avatar, &bonk_choice) {
        Ok(bonked) => bonked,
        Err(e) => {
            println!("{}", e);
            return Ok(())
        }
    };
    match bonk_image.save_with_format("bonked.png", image::ImageFormat::Png) {
        Ok(()) => {
            let _ = msg.channel_id.send_message(&ctx.http, |m| {
                m.add_file("bonked.png");
                m
            }).await;
        },
        Err(e) => {
            println!("could not save image: {}", e);
            return Ok(())
        }
    };
    Ok(())
}

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    if let Err(why) = msg.channel_id.say(&ctx.http, "Fuck you").await {
        println!("Error sending message: {:?}", why);
    }
    Ok(())
}

#[command]
async fn remind(ctx: &Context, msg: &Message) -> CommandResult {
    let parsed_time = scheduler::find_time(&msg.content);
    if let Some(parsed_time) = parsed_time {
        if let Ok(message) = msg.reply(&ctx.http, format!("I will remind you about this message on `{}` at `{}`\nother users can react with a 🕑 to also be notified", parsed_time.date(), parsed_time.time())).await {
            if let Err(why) = message.react(&ctx.http, '🕑').await {
                println!("Error! could not react to message {:?}: {}", msg, why)
            }
            let new_reminder = Reminder {
                date_time: parsed_time,
                message: msg.clone(),
                verification_message: message.clone(),
            };
            let mut data = ctx.data.write().await;
            let reminder_list = data.get_mut::<ReminderList>().expect("could not get mutable ReminderList!");
            reminder_list.push(new_reminder);
            if let Err(why) = save_reminder_list(&reminder_list) {
                println!("could not save reminder list: {}", why);
            }
        }
    }
    println!("{:?}", parsed_time);
    Ok(())
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        //println!("{}: {}", msg.author.name, msg.content);
        let mocks = check_mocker(&ctx, *msg.author.id.as_u64()).await;
        if mocks > 0 {
            //mock this user, then decrement their value in the tracker
            decrement_mocker(&ctx, *msg.author.id.as_u64()).await;
            println!("mocking user: {} ({} mocks left)", msg.author.name, mocks-1);
            let mocked_msg = mock_string(&msg.content);
            if let Err(why) = msg.channel_id.say(&ctx.http, mocked_msg).await {
                println!("Error sending message: {:?}", why);
            }
        }
        //check if enough time has elapsed since the last time we changed status
        //change at most every hour
        let time_wait = 60*60;
        let elapsed_time = check_status_time(&ctx).await;
        if elapsed_time.as_secs() > time_wait {
            update_activity(&ctx).await;
        }
        //check em
        let mut id = *msg.id.as_u64();
        let mut count = 0i32;
        loop {
            count += 1;
            let digit = id % 10;
            id /= 10;
            if digit != (id % 10) || id == 0 {
                break
            }
        }
        let emoji = match count {
            2 => {
                Some(ReactionType::Custom {
                    animated: false,
                    id: 796856698098810931.into(),
                    name: Some("dubs".to_string()),
                })
            },
            3 => {
                Some(ReactionType::Custom {
                    animated: false,
                    id: 796861409203060776.into(),
                    name: Some("trips".to_string()),
                })
            },
            4 => {
                Some(ReactionType::Custom {
                    animated: false,
                    id: 796861430297001985.into(),
                    name: Some("quads".to_string()),
                })
            },
            5 => {
                Some(ReactionType::Custom {
                    animated: false,
                    id: 796861455894577163.into(),
                    name: Some("quints".to_string()),
                })
            },
            6 => {
                Some(ReactionType::Custom {
                    animated: false,
                    id: 796861477004771328.into(),
                    name: Some("sexes".to_string()),
                })
            },
            _ => None,
        };
        match emoji {
            Some(emoji) => {
                if let Err(why) = msg.react(&ctx.http, emoji).await {
                    println!("could not react to message: {}", why);
                }
            },
            _ => (),
        }
        //ehem...culture time
        let me = match ctx.http.as_ref().get_current_user().await {
            Ok(user) => *user.id.as_u64(),
            Err(e) => {
                println!("couldn't get current user??? {}", e);
                0
            }
        };
        if let Some(channel) = msg.channel(&ctx).await {
            if channel.is_nsfw() && msg.author.id != me {
                let test_id = *msg.id.as_u64() % 1_000_000;
                let mut banned_tags: Vec<u64> = Vec::new();
                //no loli please
                banned_tags.push(19440);
                match reqwest::get(&format!("https://nhentai.net/api/gallery/{}", test_id)).await {
                    Ok(response) => {
                        if response.status() == 200 {
                            match response.text().await {
                                Ok(raw) => {
                                    match serde_json::from_str(&raw) {
                                        Ok(Value::Object(map)) => {
                                            let banned = match map.get("tags") {
                                                Some(Value::Array(array)) => {
                                                    array.iter().any( |tag| {
                                                        if let Some(Value::Number(id)) = tag.get("id") {
                                                            banned_tags.iter().any( |&banned_id| banned_id == id.as_u64().unwrap_or(0))
                                                        }
                                                        else {
                                                            false
                                                        }
                                                    })
                                                },
                                                _ => false,
                                            };
                                            if banned {
                                                println!("{} banned tag detected", test_id);
                                            }
                                            else {
                                                let title = match &map["title"]["english"] {
                                                    Value::String(title) => format!("{}\nhttps://nhentai.net/g/{}", title.as_str(), test_id),
                                                    _ => format!("https://nhentai.net/g/{}", test_id),
                                                };
                                                if let Err(why) = msg.channel_id.say(&ctx.http, title).await {
                                                    println!("Error sending message: {:?}", why);
                                                }
                                            }
                                        },
                                        Ok(_) => println!("didn't get an object back"),
                                        Err(why) => println!("could not parse json response: {}", why),
                                    }
                                },
                                Err(why) => println!("could not get text of response: {}", why),
                            }
                        }
                        else {
                            println!("{} not found", test_id);
                        }
                    },
                    Err(why) => println!("could not send request to website: {}", why),
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let ctx2 = ctx.clone();
        tokio::spawn(async move {
            let ctx = ctx2;
            let me = match ctx.http.as_ref().get_current_user().await {
                Ok(user) => *user.id.as_u64(),
                Err(e) => {
                    println!("couldn't get current user??? {}", e);
                    0
                }
            };
            //every minute, wake up and fire any reminders that have expired
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                {
                    let mut data = ctx.data.write().await;
                    let now = chrono::Local::now().naive_local();
                    let reminder_list = data.get_mut::<ReminderList>().expect("could not get mutable reminder list");
                    let expired_reminders = reminder_list.iter().filter(|reminder| reminder.date_time < now);
                    for reminder in expired_reminders {
                        let mut other_users = reminder.verification_message.reaction_users(&ctx, '🕑', None, None).await.unwrap_or(Vec::new());
                        other_users.retain(|user| *user.id.as_u64() != me);

                        if let Err(why) = reminder.message.reply_ping(&ctx, "Reminding you of this message").await {
                            println!("Error! could not post reply message: {}", why);
                        }
                        if other_users.len() > 0 {
                            let mut msg_content = MessageBuilder::new();
                            for user in other_users {
                                msg_content.mention(&user);
                            }
                            println!("{}", msg_content);
                            if let Err(why) = reminder.message.channel_id.say(&ctx, msg_content).await {
                                println!("Error! could not post reply message: {}", why);
                            }
                        }
                    }
                    reminder_list.retain(|reminder| reminder.date_time > now);
                    if let Err(why) = save_reminder_list(reminder_list) {
                        println!("could not save reminder list: {}", why);
                    }
                }
            }
        });
        update_activity(&ctx).await;
    }
}



async fn track_mocker(ctx: &Context, user: u64, amount: isize) {
    let mut data = ctx.data.write().await;
    let mock_tracker = data.get_mut::<MockTracker>().expect("could not get mutable tracker!");
    let entry = mock_tracker.entry(user).or_insert(amount);
    *entry = amount;
}

async fn decrement_mocker(ctx: &Context, user: u64) {
    let mut data = ctx.data.write().await;
    let mock_tracker = match data.get_mut::<MockTracker>() {
        Some(tracker) => tracker,
        None => {
            println!("could not get mutable reference to the mock tracker!");
            return
        }
    };
    let entry = mock_tracker.entry(user).or_insert(1);
    *entry -= 1;
    if *entry <= 0 {
        mock_tracker.remove(&user);
    }
}

async fn check_mocker(ctx: &Context, user: u64) -> isize {
    let data = ctx.data.read().await;
    let mock_tracker = match data.get::<MockTracker>() {
        Some(value) => value,
        _ => {
            println!("couldn't get tracker");
            return 0
        },
    };
    match mock_tracker.get(&user) {
        Some(amount_left) => *amount_left,
        None => 0,
    }
}

//time since last update
async fn check_status_time(ctx: &Context) -> Duration {
    let data = ctx.data.read().await;
    let last_change = match data.get::<StatusTimer>() {
        Some(value) => value,
        _ => {
            println!("couldn't get last status change!");
            return Duration::from_secs(0)
        },
    };
    Instant::now() - *last_change
}

async fn update_status_time(ctx: &Context) {
    let mut data = ctx.data.write().await;
    let status_time = match data.get_mut::<StatusTimer>() {
        Some(time) => time,
        None => {
            println!("could not get mutable reference to the status timer!");
            return
        },
    };
    *status_time = Instant::now();
}

async fn update_activity(ctx: &Context) {
    match std::fs::read_to_string("./assets/activities.json") {
        Ok(string) => match serde_json::from_str::<Vec<String>>(&string) {
            Ok(data) => {
                let choice = data.choose(&mut rand::thread_rng());
                match choice {
                    Some(item) => {
                        //let mut activity = Activity::playing("Custom Status");
                        //activity.kind = ActivityType::Custom;
                        //activity.state = Some(item.to_string());
                        //ctx.set_activity(activity);
                        ctx.set_activity(Activity::playing(item)).await;
                    },
                    None => println!("activities has no elements to choose from"),
                }
            },
            Err(e) => println!("could not parse JSON: {}", e),
        },
        Err(e) => println!("could not read 'activities.json': {}", e),
    };
    update_status_time(&ctx).await;
}

fn mock_string(to_mock: &str) -> String {
    let mut rng = rand::thread_rng();
    to_mock.chars().map(|ch| {
        if rng.gen::<u8>() % 2 == 0 {
            if ch.is_uppercase() {
                ch.to_lowercase().collect::<String>()
            }
            else {
                ch.to_uppercase().collect::<String>()
            }
        }
        else {
            ch.to_string()
        }
    }).collect()
}



//serialize the reminder list
fn save_reminder_list(reminder_list: &Vec<Reminder>) -> Result<(), Box<dyn Error>> {
    let json_content = serde_json::to_string(&reminder_list)?;
    std::fs::write("./assets/reminder_list.json", json_content)?;
    Ok(())
}