/// Renders the parent invite email HTML.
/// Maps to the Supabase "Confirm Sign Up" template used for parent enrollment invitations.
pub fn parent_invite_html(first_name: &str, last_name: &str, confirmation_url: &str) -> String {
    format!(
        r#"<html>
  <body style="font-family: Arial, sans-serif; line-height: 1; color: #333;">
    <div style="max-width: 500px; margin: auto; padding: 20px; border: 1px solid #e0e0e0; border-radius: 8px;">
      <p>Dear {first_name} {last_name},</p>
      <p>We hope this message finds you well. We are pleased to inform you that your enrollment request at <strong>The Goddard School</strong>
has been received and approved for the next stage of the admission process.</p>
      <p style="text-align: center;">
        <a href="{url}" style="display: inline-block; padding: 10px 20px; margin: 10px 0; background-color: #4CAF50; color:
white; text-decoration: none; border-radius: 5px;">Confirm Your Email</a>
      </p>
      <p>Thank you for choosing <strong>The Goddard School</strong>.</p>
      <p>Warm regards,<br>Admin Team,<br><strong>The Goddard School</strong></p>
    </div>
  </body>
</html>"#,
        first_name = first_name,
        last_name = last_name,
        url = confirmation_url,
    )
}

/// Renders the admin / SuperAdmin invite email HTML.
/// Maps to the Supabase "Invite User" template used for Admin and SuperAdmin invitations.
pub fn admin_invite_html(
    first_name: &str,
    last_name: &str,
    school_name: &str,
    confirmation_url: &str,
    token: &str,
) -> String {
    format!(
        r#"<html>
  <body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: auto; padding: 30px; border: 1px solid #e0e0e0; border-radius: 8px; background-color: #f9f9f9;">
      <h2 style="color: #2c3e50; margin-top: 0;">Welcome to {school_name}</h2>

      <p>Dear {first_name} {last_name},</p>

      <p>You have been designated as the <strong>School Administrator</strong> for <strong>{school_name}</strong>. This grants you full access
to manage your school's operations, staff, and enrollment processes.</p>

      <div style="background-color: #fff; padding: 20px; border-radius: 5px; margin: 20px 0;">
        <p style="margin: 0 0 15px 0;"><strong>To get started:</strong></p>
        <ol style="margin: 0; padding-left: 20px;">
          <li>Click the button below to confirm your email and set your password</li>
          <li>Complete your administrator profile</li>
          <li>Begin managing your school's enrollment and staff</li>
        </ol>
      </div>

      <p style="text-align: center; margin: 30px 0;">
        <a href="{url}" style="display: inline-block; padding: 15px 30px; background-color: #3498db; color: white; text-decoration:
none; border-radius: 5px; font-weight: bold; font-size: 16px;">Set Your Password</a>
      </p>

      <p style="font-size: 14px; color: #666;">If the button doesn't work, copy and paste this link into your browser:</p>
      <p style="font-size: 14px; color: #3498db; word-break: break-all;">{url}</p>

      <p style="font-size: 14px; color: #666; margin-top: 20px;">Or use this confirmation code: <strong style="color: #333;">{token}</strong></p>

      <hr style="border: none; border-top: 1px solid #e0e0e0; margin: 30px 0;">

      <p style="font-size: 14px; color: #666; margin-bottom: 5px;">If you did not request this access or believe you received this email in error,
please contact our support team immediately.</p>

      <p style="margin-top: 30px;">Best regards,<br>
      <strong>{school_name} Support Team</strong></p>
    </div>

    <p style="text-align: center; font-size: 12px; color: #999; margin-top: 20px;">
      This is an automated message. Please do not reply to this email.
    </p>
  </body>
</html>"#,
        school_name = school_name,
        first_name = first_name,
        last_name = last_name,
        url = confirmation_url,
        token = token,
    )
}
