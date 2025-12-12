use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Ko-Fi membership email"]
pub struct ClaimMembershipModal {
    #[name = "Email"]
    #[placeholder = "Email used on Ko-Fi"]
    pub email: String,
}