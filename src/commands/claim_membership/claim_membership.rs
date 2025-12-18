use poise::{serenity_prelude::Error, Modal};

use crate::commands::claim_membership::modal::ClaimMembershipModal;
use crate::commands::claim_membership::utils::{
    claim_membership_by_id, find_latest_unclaimed_membership_event, get_memberships_collection,
    get_users_collection, normalize_email,
};
use crate::commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion};

/// 📂 Claims membership using the email used to buy the membership on Ko-Fi
///
/// Usage: `/claim_membership`
#[poise::command(slash_command)]
pub async fn claim_membership(
    ctx: poise::ApplicationContext<'_, crate::Data, Error>,
) -> Result<(), Error> {
    let modal_data: ClaimMembershipModal = match ClaimMembershipModal::execute(ctx).await {
        Ok(Some(data)) => data,
        Ok(None) => return Ok(()),
        Err(err) => {
            let error_message = format!("Error executing the modal: {:?}", err);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let email_raw = modal_data.email.as_str();
    let email = normalize_email(email_raw);

    // basic format validation AFTER normalize
    let email_regex =
        regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if !email_regex.is_match(&email) {
        let reply = ctx
            .send(create_embed_error(
                "Invalid email format. Please enter a valid email address.",
            ))
            .await?;
        schedule_message_deletion(reply, ctx).await?;
        return Ok(());
    }

    let memberships = get_memberships_collection()
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    // Find latest unclaimed subscription-payment event for this email
    let event = find_latest_unclaimed_membership_event(&memberships, &email)
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, format!("{e:?}"))))?;

    let Some(event_doc) = event else {
        let reply = ctx
            .send(create_embed_error(
                "No unclaimed subscription payment found for this email. \
Make sure you used the same email on Ko-fi and that your subscription payment went through.",
            ))
            .await?;
        schedule_message_deletion(reply, ctx).await?;
        return Ok(());
    };

    // Extract _id
    let id = match event_doc.get_object_id("_id") {
        Ok(id) => id.to_owned(),
        Err(_) => {
            let reply = ctx
                .send(create_embed_error(
                    "Internal error: membership record has no valid _id.",
                ))
                .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    // Atomic claim
    let now = chrono::Utc::now().timestamp();
    let discord_id = ctx.author().id.to_string();

    let claimed = claim_membership_by_id(&memberships, id, &discord_id, now)
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, format!("{e:?}"))))?;

    if !claimed {
        let reply = ctx
            .send(create_embed_error(
                "This membership was just claimed already (or is no longer claimable).",
            ))
            .await?;
        schedule_message_deletion(reply, ctx).await?;
        return Ok(());
    }

    // Upsert user
    let users = get_users_collection()
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let user_filter = mongodb::bson::doc! { "email": &email };
    let existing_user = users
        .find_one(user_filter.clone())
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, format!("{e:?}"))))?;

    // Optional: prevent claiming same email on a different discord account
    if let Some(doc) = &existing_user {
        if let Ok(existing_discord) = doc.get_str("discord_id") {
            if existing_discord != discord_id {
                let reply = ctx
                    .send(create_embed_error(
                        "This email is already linked to another Discord account. \
If you need to transfer it, please contact support.",
                    ))
                    .await?;
                schedule_message_deletion(reply, ctx).await?;
                return Ok(());
            }
        }
    }

    if existing_user.is_none() {
        let new_user = mongodb::bson::doc! {
            "discord_id": &discord_id,
            "email": &email,
            "created_at": now,
            "updated_at": now,
            "membership_status": "active",
            "membership_claimed_at": now,
            "commands": [],
        };

        users.insert_one(new_user)
            .await
            .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, format!("{e:?}"))))?;
    } else {
        let user_update = mongodb::bson::doc! {
            "$set": {
                "discord_id": &discord_id,
                "membership_status": "active",
                "membership_claimed_at": now,
                "updated_at": now,
            }
        };

        users.update_one(user_filter, user_update)
            .await
            .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, format!("{e:?}"))))?;
    }

    let discord_username = ctx.author().name.clone();
    ctx.say(format!("✅ Subscription linked for your Discord account **{}**", discord_username)).await?;
    Ok(())
}
