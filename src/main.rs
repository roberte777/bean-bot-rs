use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serenity::model::gateway::Ready;
use serenity::model::prelude::component::ButtonStyle;
use serenity::model::prelude::{Emoji, ReactionType};
use serenity::utils::MessageBuilder;
use std::env;

use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{Args, CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;

#[group]
#[commands(bean, wager, bean_user, register)]
struct General;

struct Handler;
#[derive(Serialize)]
struct UserWagerSend {
    discord_id: u64,
    user_name: String,
    wager_id: i32,
}

#[derive(Serialize)]
struct CloseWagerSend {
    wager_id: i32,
    winning_user_discord_ids: Vec<u64>,
    losing_user_discord_ids: Vec<u64>,
}

#[derive(Serialize)]
struct RemoveUserWagerSend {
    wager_id: i32,
    discord_id: u64,
}
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
    async fn reaction_remove(
        &self,
        ctx: Context,
        add_reaction: serenity::model::channel::Reaction,
    ) {
        if add_reaction
            .message(&ctx)
            .await
            .expect("excpeted to be able to find the message")
            .author
            .bot
        {
            if add_reaction.emoji.as_data().as_str() == "❤️" {
                let message = add_reaction.message(&ctx).await.unwrap();
                let mut wager_id = 0;
                for line in message.content.lines() {
                    if line.contains("Your wager id is:") {
                        wager_id = line.split(":").last().unwrap().trim().parse().unwrap();
                    }
                }
                if wager_id == 0 {
                    return;
                }
                let discord_id = u64::from(add_reaction.user_id.unwrap());
                // send the delete request
                let mut url = env::var("BEAN_BUCKS_URL").expect("URL");
                url.push_str("/user/wager");
                let client = reqwest::Client::new();
                let res = client
                    .delete(&url)
                    .json(&RemoveUserWagerSend {
                        wager_id,
                        discord_id,
                    })
                    .send()
                    .await;
                match res {
                    Ok(res) => {
                        if res.status().is_success() {
                            println!("Successfully removed wager!");
                        } else {
                            println!(
                                "Failed to remove user from wager! Status: {}\nText: {}",
                                res.status(),
                                res.text().await.unwrap()
                            );
                        }
                    }
                    Err(e) => {
                        println!("Failed to remove user from wager! Error: {}", e);
                    }
                }
            }
        }
    }
    // handle reactions
    async fn reaction_add(&self, ctx: Context, add_reaction: serenity::model::channel::Reaction) {
        let mut url = env::var("BEAN_BUCKS_URL").expect("URL");
        let client = reqwest::Client::new();
        // check if message is from the bot
        if add_reaction
            .message(&ctx)
            .await
            .expect("excpeted to be able to find the message")
            .author
            .bot
            && !add_reaction.user(&ctx).await.unwrap().bot
        {
            // if message is a heart
            if add_reaction.emoji.as_data().as_str() == "❤️" {
                // check if the message is a wager message
                // it is a wager message if a line has the string: "Your wager id is:"
                let message = add_reaction.message(&ctx).await.unwrap();
                let mut wager_id = 0;
                for line in message.content.lines() {
                    if line.contains("Your wager id is:") {
                        wager_id = line.split(":").last().unwrap().trim().parse().unwrap();
                    }
                }
                if wager_id == 0 {
                    return;
                }
                let discord_id = u64::from(add_reaction.user_id.unwrap());
                let user_name = add_reaction
                    .user(&ctx)
                    .await
                    .expect("expected to be able to find the user")
                    .name;

                url.push_str("/user/wager");
                let res = client
                    .post(&url)
                    .json(&UserWagerSend {
                        discord_id,
                        user_name,
                        wager_id,
                    })
                    .send()
                    .await;
                match res {
                    Ok(res) => {
                        if res.status().is_success() {
                            println!("Successfully joined wager!");
                        } else {
                            println!(
                                "Failed to join wager! Status: {}\nText: {}",
                                res.status(),
                                res.text().await.unwrap()
                            );
                            add_reaction.delete(&ctx).await.unwrap();
                        }
                    }
                    Err(e) => {
                        println!("Failed to join wager! Error: {}", e);
                    }
                }
            }
            //if reaction is green checkmark, handle
            if add_reaction.emoji.as_data().as_str() == "✅" {
                // check if the message is a wager message
                // it is a wager message if a line has the string: "Your wager id is:"
                let message = add_reaction.message(&ctx).await.unwrap();
                let mut wager_id = 0;
                for line in message.content.lines() {
                    if line.contains("Your wager id is:") {
                        wager_id = line.split(":").last().unwrap().trim().parse().unwrap();
                    }
                }
                if wager_id == 0 {
                    return;
                }

                //get the discord id's of the winners (the people who reacted with a thumbs up)
                //remove bot from list of winning and losing users
                let mut winning_users = message
                    .reaction_users(&ctx, "👍".chars().last().unwrap(), None, None)
                    .await
                    .unwrap();
                winning_users.retain(|user| !user.bot);

                let winning_user_ids = winning_users
                    .iter()
                    .map(|user| u64::from(user.id))
                    .collect();

                let mut losing_users = message
                    .reaction_users(&ctx, "👎".chars().last().unwrap(), None, None)
                    .await
                    .unwrap();
                losing_users.retain(|user| !user.bot);
                let losing_user_ids = losing_users.iter().map(|user| u64::from(user.id)).collect();

                url.push_str("/wager");
                println!("url: {}", url);
                let client = reqwest::Client::new();
                let res = client
                    .patch(&url)
                    .json(&CloseWagerSend {
                        wager_id,
                        winning_user_discord_ids: winning_user_ids,
                        losing_user_discord_ids: losing_user_ids,
                    })
                    .send()
                    .await;

                let winning_string = winning_users
                    .iter()
                    .map(|user| user.name.clone())
                    .collect::<Vec<String>>()
                    .join("\n");
                let losing_string = losing_users
                    .iter()
                    .map(|user| user.name.clone())
                    .collect::<Vec<String>>()
                    .join("\n");

                match res {
                    Ok(res) => {
                        if res.status().is_success() {
                            add_reaction
                                .message(&ctx)
                                .await
                                .unwrap()
                                .reply(
                                    &ctx,
                                    format!(
                                        "Successfully closed wager {}\nWinners:\n{}\nLosers:\n{}",
                                        wager_id, winning_string, losing_string
                                    ),
                                )
                                .await
                                .expect("expected to be able to reply to message");
                        } else {
                            //respond to message saying it failed to close, take off the green
                            //checkmark
                            println!(
                                "Failed to close wager! Status: {}\nText: {}",
                                res.status(),
                                res.text().await.unwrap()
                            );
                            add_reaction
                                .delete(&ctx)
                                .await
                                .expect("expected to be able to remove the reaction");
                            add_reaction
                                .message(&ctx)
                                .await
                                .unwrap()
                                .reply(&ctx, "Failed to close wager!")
                                .await
                                .expect("expected to be able to reply to message");
                        }
                    }
                    Err(_) => {
                        add_reaction
                            .delete(&ctx)
                            .await
                            .expect("expected to be able to remove the reaction");
                        add_reaction
                            .message(&ctx)
                            .await
                            .unwrap()
                            .reply(&ctx, "Failed to close wager! API could be offline")
                            .await
                            .expect("expected to be able to reply to message");
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    // https://docs.rs/serenity/0.11.5/serenity/framework/standard/struct.StandardFramework.html
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("/").with_whitespace(true)) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn bean(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "FAT!").await?;

    Ok(())
}

#[derive(Serialize)]
struct WagerSend {
    amount: i32,
}
#[derive(Deserialize)]
struct WagerReceive {
    id: i32,
    amount: i32,
    closed: bool,
}
#[command]
async fn wager(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    match args.single_quoted::<i32>() {
        Ok(amount) => {
            let mut url = env::var("BEAN_BUCKS_URL").expect("URL");
            url.push_str("/wager");
            let client = reqwest::Client::new();
            let res = client.post(&url).json(&WagerSend { amount }).send().await?;
            if res.status().is_success() {
                let wager: WagerReceive = res.json().await?;
                msg
                    .channel_id
                    .send_message(&ctx, |m| {
                        m.reference_message(msg);
                        m.content(
                    format!("Wager successful! \nYour wager id is: {}\nWager amount: {}\nWager Status: Open", wager.id, wager.amount)
                        )
                            .reactions([
                                ReactionType::Unicode("❤️".to_string()),
                                ReactionType::Unicode("👍".to_string()),
                                ReactionType::Unicode("👎".to_string()),
                                ReactionType::Unicode("✅".to_string()),
                            ])
                    })

                    .await
                    .expect("expected to be able to send message");
            } else {
                println!(
                    "Failed to create wager! Status: {}\nText: {}",
                    res.status(),
                    res.text().await?
                );
                msg.reply(ctx, "Faield to create wager!").await?;
            }
        }
        Err(_) => {
            msg.reply(ctx, "You must wager an amount!").await?;
        }
    };

    Ok(())
}
#[derive(Deserialize)]
struct User {
    id: i32,
    discord_id: u64,
    user_name: String,
    bucks: i32,
}
#[command]
async fn bean_user(ctx: &Context, msg: &Message) -> CommandResult {
    let user = msg.mentions.first();
    if user.is_none() {
        msg.reply(ctx, "You must mention a user!").await?;
        return Ok(());
    }
    let user_id = user.unwrap().id.0;
    let mut url = env::var("BEAN_BUCKS_URL").expect("URL");
    url.push_str("/user");
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[("discord_id", user_id)])
        .send()
        .await?;
    if res.status().is_success() {
        let user: User = res.json().await?;
        msg.reply(
            ctx,
            format!(
                "User found! \nUser id: {}\nDiscord id: {}\nUser name: {}\nUser balance: {}",
                user.id, user.discord_id, user.user_name, user.bucks
            ),
        )
        .await?;
    } else {
        println!(
            "Failed to find user! Status: {}\nText: {}",
            res.status(),
            res.text().await?
        );
        msg.reply(ctx, "Failed to find user!").await?;
    }

    Ok(())
}
#[derive(Serialize)]
struct RegisterUser {
    discord_id: u64,
    user_name: String,
}
#[command]
async fn register(ctx: &Context, msg: &Message) -> CommandResult {
    let user = RegisterUser {
        discord_id: msg.author.id.into(),
        user_name: msg.author.name.clone(),
    };
    let mut url = env::var("BEAN_BUCKS_URL").expect("URL");
    url.push_str("/user");
    let client = reqwest::Client::new();
    let res = client.post(&url).json(&user).send().await?;
    if res.status().is_success() {
        msg.reply(ctx, "User registered!").await?;
    } else {
        println!(
            "Failed to register user! Status: {}\nText: {}",
            res.status(),
            res.text().await?
        );
        msg.reply(ctx, "Failed to register user!").await?;
    }
    Ok(())
}
