use email_format::Email;
use validator::{validate_email, ValidationError};

pub fn email_vec(emails: &Vec<String>) -> Result<(), ValidationError> {
    for email in emails {
        if !validate_email(email) {
            return Err(ValidationError::new("vec contains invalid email"));
        }
    }

    Ok(())
}

pub fn rfc_5322_email(email: &str) -> Result<(), ValidationError> {
    if Email::new(email, "Wed, 5 Jan 2015 15:13:05 +1300").is_err() {
        return Err(ValidationError::new("sender is not a valid RFC5322 email"));
    }

    Ok(())
}
