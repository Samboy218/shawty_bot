# shawty_bot
Discord bot written in rust, using [serenity](https://github.com/serenity-rs/serenity)

## Commands

### !mock <target_user>
tracks the mentioned user, and the next time they send a message in a channel shawty_bot can see, shawty_bot will repeat what they said iN SPOngEbob TexT

### !bonk <target_user>
overlays the target user's profile picture with one of the images specified in assets/bonk_locations.json. All coordinates are center points.
- name String: the filename of the bonk image
- bonkee_x u32: the x co-ordinate of the target's profile picture
- bonkee_y u32: the y co-ordinate of the target's profile picture
- bonkee_width u32: stretch/shrink the target's profile picture's width to match this value
- bonkee_height u32: stretch/shrink the target's profile picture's height to match this value
- bonk_label_x u32: the x co-ordinate of the bonk label (can place this out-of-bounds to get rid of it)
- bonk_label_y u32: the y co-ordinate of the bonk label (can place this out-of-bounds to get rid of it)
- bonk_label_width u32: stretch/shrink the bonk label's width to match this value
- bonk_label_height u32: stretch/shrink the bonk label's height to match this value
- bonkee_top bool: if true, the bonkee will be put on top of the bonk image. if false the bonkee will be put on the bottom layer. This is useful if your bonk image has transparency.

### !remind
Attempts to find a datetime in the message, and if it can it will message the user again at that time. Others can join in the reminder by reacting.  
Ambiguous times are resolved on a best effort basis, and the bot will only attempt to resolve times that are in the future.
the bot will attempt to find dates/times in the following formats (and will prioritize resolved datetimes in this order)
- Exact date: 2021/06/11 03/15/2021 10/02/21 05/27 
- Exact time: 4:33 18:30 5:45pm 5:50am 5:50 a.m.
- Relative offset: '3 days' '5 hours' '47 minutes' '5 weeks' '2 years' '2348103 milliseconds' 'next week' 'next month'
- 'fuzzy' time resolution uses [this library](https://github.com/isaacrlee/event-parser) to attempt to catch any other weird formats humans may use

##Misc behavior
- shawty_bot will periodically (about every hour) change it's activity to one of the ones defined in assets/activities.json
- shawty_bot will examine all message id's in channels it can see, if it encounters a message id with repeating final digits, it will add an approprite reaction based on how many digits repeat
