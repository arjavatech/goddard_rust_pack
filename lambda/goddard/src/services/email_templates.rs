use crate::models::email::{
    ChildAddedNotification, ChildArchivedNotification, FormApprovedNotification,
    FormAssignedNotification, FormRejectedNotification, ParentDeactivatedNotification,
};
use chrono::{DateTime, NaiveDate, Utc};

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
    )
}

// =====================================================
// Parent lifecycle notification templates
// See docs/EMAIL_NOTIFICATIONS.md
// =====================================================

/// Shared visual shell used by all parent lifecycle notifications.
/// Keeps header/footer/typography consistent and lets each template focus on its body content.
fn render_shell(
    accent: &str,
    headline: &str,
    preheader: &str,
    body_html: &str,
    cta_block: &str,
) -> String {
    format!(
        r#"<html>
  <body style="margin:0;padding:0;background-color:#f9f9f9;font-family:Arial,Helvetica,sans-serif;color:#333;line-height:1.6;">
    <div style="display:none;max-height:0;overflow:hidden;font-size:1px;color:#f9f9f9;">{preheader}</div>

    <div style="max-width:600px;margin:0 auto;padding:30px 20px;">
      <div style="background:#ffffff;border:1px solid #e0e0e0;border-radius:8px;padding:32px;">
        <div style="height:4px;background:{accent};border-radius:4px;margin-bottom:24px;"></div>

        <h2 style="color:#2c3e50;margin:0 0 16px 0;font-size:20px;">{headline}</h2>

        {body}

        {cta}

        <hr style="border:none;border-top:1px solid #e0e0e0;margin:28px 0;">

        <p style="margin:0;color:#666;font-size:14px;">
          Warm regards,<br>
          <strong>The Goddard School Team</strong>
        </p>
      </div>

      <p style="text-align:center;color:#999;font-size:12px;margin-top:16px;">
        This is an automated message. Please do not reply to this email.
      </p>
    </div>
  </body>
</html>"#,
        accent = accent,
        headline = headline,
        preheader = html_escape(preheader),
        body = body_html,
        cta = cta_block,
    )
}

fn cta_button(label: &str, url: &str, color: &str) -> String {
    format!(
        r#"<p style="text-align:center;margin:28px 0 0 0;">
            <a href="{url}" style="display:inline-block;padding:14px 28px;background:{color};color:#ffffff;text-decoration:none;border-radius:5px;font-weight:bold;font-size:15px;">{label}</a>
          </p>"#,
        url = url,
        color = color,
        label = label,
    )
}

fn details_card(rows: &[(&str, String)]) -> String {
    let row_html: String = rows
        .iter()
        .map(|(label, value)| {
            format!(
                r#"<tr>
                    <td style="padding:6px 12px 6px 0;color:#666;font-size:14px;white-space:nowrap;"><strong>{label}</strong></td>
                    <td style="padding:6px 0;color:#333;font-size:14px;">{value}</td>
                  </tr>"#,
                label = html_escape(label),
                value = html_escape(value),
            )
        })
        .collect();

    format!(
        r#"<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="width:100%;background:#f7f9fb;border:1px solid #e0e0e0;border-radius:6px;padding:16px;margin:16px 0;">
            <tbody>{rows}</tbody>
          </table>"#,
        rows = row_html,
    )
}

fn html_escape(input: impl AsRef<str>) -> String {
    input
        .as_ref()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn fmt_datetime(dt: DateTime<Utc>) -> String {
    dt.format("%B %d, %Y at %I:%M %p UTC").to_string()
}

fn fmt_date_only(d: Option<NaiveDate>) -> String {
    match d {
        Some(date) => date.format("%B %d, %Y").to_string(),
        None => "Not provided".to_string(),
    }
}

// ----- Template 1: Form Approved -----

pub fn form_approved_html(payload: &FormApprovedNotification) -> String {
    let accent = "#27ae60"; // success green
    let headline = format!(
        "Good news — {} has been approved",
        html_escape(&payload.form_name)
    );
    let preheader = format!(
        "The {} you submitted for {} has been reviewed and approved by your Goddard School team.",
        payload.form_name, payload.child_name
    );

    let details = details_card(&[
        ("Form", payload.form_name.clone()),
        ("Child", payload.child_name.clone()),
        ("Status", "Approved".to_string()),
        ("Reviewed on", fmt_datetime(payload.reviewed_on)),
        ("Reviewer", payload.reviewer_name.clone()),
    ]);

    let notes_block = match payload.notes.as_ref().filter(|s| !s.trim().is_empty()) {
        Some(notes) => format!(
            r#"<div style="background:#f1f8f4;border-left:4px solid {accent};padding:12px 16px;margin:16px 0;border-radius:4px;">
                <p style="margin:0 0 6px 0;font-size:13px;color:#27ae60;text-transform:uppercase;letter-spacing:0.5px;"><strong>Notes from your reviewer</strong></p>
                <p style="margin:0;color:#333;font-size:14px;">{notes}</p>
              </div>"#,
            accent = accent,
            notes = html_escape(notes),
        ),
        None => String::new(),
    };

    let body = format!(
        r#"<p>Dear {first_name},</p>
          <p>Great news — the <strong>{form}</strong> you submitted for <strong>{child}</strong> has been reviewed and approved by the Goddard School team.</p>
          {details}
          {notes}
          <p style="margin-top:20px;">No further action is needed on this form. You can view all of your child's forms anytime from your parent dashboard.</p>
          <p style="margin-top:8px;color:#666;font-size:14px;">If you have any questions, please reach out to your school's administrator.</p>"#,
        first_name = html_escape(&payload.parent_first_name),
        form = html_escape(&payload.form_name),
        child = html_escape(&payload.child_name),
        details = details,
        notes = notes_block,
    );

    let cta = cta_button("Open Parent Dashboard", &payload.dashboard_url, "#3498db");

    render_shell(accent, &headline, &preheader, &body, &cta)
}

// ----- Template 2: Form Rejected -----

pub fn form_rejected_html(payload: &FormRejectedNotification) -> String {
    let accent = "#e67e22"; // warning orange
    let headline = format!(
        "Action needed: {} requires updates",
        html_escape(&payload.form_name)
    );
    let preheader = format!(
        "Please review the notes and resubmit the {} for {}.",
        payload.form_name, payload.child_name
    );

    let details = details_card(&[
        ("Form", payload.form_name.clone()),
        ("Child", payload.child_name.clone()),
        ("Status", "Needs Updates".to_string()),
        ("Reviewed on", fmt_datetime(payload.reviewed_on)),
        ("Reviewer", payload.reviewer_name.clone()),
    ]);

    let notes_text = payload
        .notes
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Please contact your school administrator for details.".to_string());

    let body = format!(
        r#"<p>Dear {first_name},</p>
          <p>Thank you for submitting the <strong>{form}</strong> for <strong>{child}</strong>. After review, our team has asked for a few updates before the form can be approved.</p>
          {details}
          <div style="background:#fdf3e7;border-left:4px solid {accent};padding:12px 16px;margin:16px 0;border-radius:4px;">
            <p style="margin:0 0 6px 0;font-size:13px;color:#b9651b;text-transform:uppercase;letter-spacing:0.5px;"><strong>What needs attention</strong></p>
            <p style="margin:0;color:#333;font-size:14px;">{notes}</p>
          </div>
          <p style="margin:18px 0 6px 0;"><strong>Next steps</strong></p>
          <ol style="margin:0;padding-left:20px;color:#333;">
            <li>Open your parent dashboard</li>
            <li>Locate <strong>{form}</strong> under <strong>{child}</strong></li>
            <li>Make the requested updates and resubmit</li>
          </ol>
          <p style="margin-top:18px;color:#666;font-size:14px;">If anything is unclear, your school's administrator is happy to help.</p>"#,
        first_name = html_escape(&payload.parent_first_name),
        form = html_escape(&payload.form_name),
        child = html_escape(&payload.child_name),
        details = details,
        notes = html_escape(&notes_text),
        accent = accent,
    );

    let cta = cta_button("Update Form Now", &payload.dashboard_url, accent);

    render_shell(accent, &headline, &preheader, &body, &cta)
}

// ----- Template 3: Additional Child Added -----

pub fn child_added_html(payload: &ChildAddedNotification) -> String {
    let accent = "#3498db";
    let headline = format!(
        "{} has been added to your account",
        html_escape(&payload.child_name)
    );
    let preheader = format!(
        "{} is now enrolled in {}. {} new forms are ready for you.",
        payload.child_name, payload.classroom_name, payload.form_count
    );

    let details = details_card(&[
        ("Child", payload.child_name.clone()),
        ("Date of birth", fmt_date_only(payload.child_dob)),
        ("Classroom", payload.classroom_name.clone()),
        ("School", payload.school_name.clone()),
        ("Added on", fmt_datetime(payload.added_on)),
    ]);

    let forms_line = if payload.form_count == 0 {
        "No forms have been assigned yet — your administrator will let you know if any are needed."
            .to_string()
    } else if payload.form_count == 1 {
        format!(
            "We've assigned <strong>1 enrollment form</strong> for {}. To get your child fully enrolled, please log in to your parent dashboard and complete it at your earliest convenience.",
            html_escape(&payload.child_name)
        )
    } else {
        format!(
            "We've assigned <strong>{} enrollment forms</strong> for {}. To get your child fully enrolled, please log in to your parent dashboard and complete them at your earliest convenience.",
            payload.form_count,
            html_escape(&payload.child_name)
        )
    };

    let body = format!(
        r#"<p>Dear {first_name},</p>
          <p>We're delighted to confirm that <strong>{child}</strong> has been added to your Goddard School account.</p>
          {details}
          <p style="margin:18px 0 6px 0;"><strong>What's next</strong></p>
          <p style="margin-top:6px;">{forms_line}</p>
          <p style="margin-top:14px;color:#555;">You'll continue to use the same login you already use for your other child(ren) — everything will appear together in one place.</p>
          <p style="margin-top:14px;color:#666;font-size:14px;">If you have any questions, please reach out to your school's administrator.</p>"#,
        first_name = html_escape(&payload.parent_first_name),
        child = html_escape(&payload.child_name),
        details = details,
        forms_line = forms_line,
    );

    let cta = if payload.form_count > 0 {
        cta_button("Complete Enrollment Forms", &payload.dashboard_url, accent)
    } else {
        cta_button("Open Parent Dashboard", &payload.dashboard_url, accent)
    };

    render_shell(accent, &headline, &preheader, &body, &cta)
}

// ----- Template 4: Parent Account Deactivated -----

pub fn parent_deactivated_html(payload: &ParentDeactivatedNotification) -> String {
    let accent = "#7f8c8d"; // muted slate
    let headline = "Your parent account has been deactivated".to_string();
    let preheader = format!(
        "Your account access at {} has been deactivated. Contact your school administrator if this was unexpected.",
        payload.school_name
    );

    let details = details_card(&[
        ("Account", payload.parent_full_name.clone()),
        ("School", payload.school_name.clone()),
        ("Status", "Deactivated".to_string()),
        ("Effective", fmt_datetime(payload.deactivated_on)),
        ("Children", payload.children_count.to_string()),
        ("Enrollments", payload.enrollments_count.to_string()),
    ]);

    let body = format!(
        r#"<p>Dear {first_name},</p>
          <p>We're writing to let you know that your parent account at <strong>{school}</strong> has been deactivated by your school administrator.</p>
          {details}
          <p style="margin:18px 0 6px 0;"><strong>What this means</strong></p>
          <ul style="margin:0;padding-left:20px;color:#333;">
            <li>You will no longer be able to sign in to the parent dashboard</li>
            <li>Your child(ren)'s active enrollments have been deactivated</li>
            <li>Any pending forms will be paused</li>
          </ul>
          <p style="margin-top:18px;">If you believe this was done in error, or you would like to discuss next steps, please contact your school administrator as soon as possible.</p>
          <p style="margin-top:14px;color:#555;">We appreciate the time you've spent as part of the Goddard School community.</p>"#,
        first_name = html_escape(&payload.parent_first_name),
        school = html_escape(&payload.school_name),
        details = details,
    );

    render_shell(accent, &headline, &preheader, &body, "")
}

// ----- Template 6: New Form Assigned -----

pub fn form_assigned_html(payload: &FormAssignedNotification) -> String {
    let accent = "#3498db"; // informational blue
    let headline = format!(
        "New form for {}: {}",
        html_escape(&payload.child_name),
        html_escape(&payload.form_name),
    );
    let preheader = format!(
        "A new {} has been assigned for {}. Please complete it from your parent dashboard.",
        payload.form_name, payload.child_name
    );

    let due_text = match payload.due_date {
        Some(date) => date.format("%B %d, %Y").to_string(),
        None => "At your earliest convenience".to_string(),
    };
    let requirement_text = if payload.is_required { "Required" } else { "Optional" };

    let details = details_card(&[
        ("Form", payload.form_name.clone()),
        ("Child", payload.child_name.clone()),
        ("School", payload.school_name.clone()),
        ("Requirement", requirement_text.to_string()),
        ("Due", due_text),
        ("Assigned on", fmt_datetime(payload.assigned_on)),
    ]);

    let body = format!(
        r#"<p>Dear {first_name},</p>
          <p>A new form has been assigned for <strong>{child}</strong>. Please take a moment to review and complete the <strong>{form}</strong> from your parent dashboard.</p>
          {details}
          <p style="margin:18px 0 6px 0;"><strong>How to complete the form</strong></p>
          <ol style="margin:0;padding-left:20px;color:#333;">
            <li>Open your parent dashboard</li>
            <li>Locate <strong>{form}</strong> under <strong>{child}</strong></li>
            <li>Fill in the requested information and submit</li>
          </ol>
          <p style="margin-top:18px;color:#666;font-size:14px;">If you have any questions about this form, please reach out to your school's administrator.</p>"#,
        first_name = html_escape(&payload.parent_first_name),
        child = html_escape(&payload.child_name),
        form = html_escape(&payload.form_name),
        details = details,
    );

    let cta = cta_button("Open Parent Dashboard", &payload.dashboard_url, accent);

    render_shell(accent, &headline, &preheader, &body, &cta)
}

// ----- Template: Bulk Import Welcome -----

pub fn bulk_import_welcome_html(
    first_name: &str,
    last_name: &str,
    email: &str,
    password: &str,
    school_name: &str,
    dashboard_url: &str,
) -> String {
    let accent = "#2ecc71"; // welcoming green
    let headline = format!("Welcome to {} — Your Account Is Ready", html_escape(school_name));
    let preheader = format!(
        "Your Goddard School parent account at {} has been created. Find your login details inside.",
        school_name
    );

    let credentials = details_card(&[
        ("Email", email.to_string()),
        ("Password", password.to_string()),
    ]);

    let body = format!(
        r#"<p>Dear {first_name} {last_name},</p>
          <p>Welcome! Your parent account at <strong>{school}</strong> has been created. You can now log in to your parent dashboard using the credentials below.</p>
          {credentials}
          <p style="margin-top:18px;"><strong>Next steps</strong></p>
          <ol style="margin:0;padding-left:20px;color:#333;">
            <li>Log in with the email and password above</li>
            <li>We recommend changing your password after your first login</li>
            <li>Complete any enrollment forms assigned to your child(ren)</li>
          </ol>
          <p style="margin-top:18px;color:#666;font-size:14px;">If you have any questions, please reach out to your school's administrator.</p>"#,
        first_name = html_escape(first_name),
        last_name = html_escape(last_name),
        school = html_escape(school_name),
        credentials = credentials,
    );

    let cta = cta_button("Log In to Parent Dashboard", dashboard_url, accent);

    render_shell(accent, &headline, &preheader, &body, &cta)
}

// ----- Template 5: Child Archived -----

pub fn child_archived_html(payload: &ChildArchivedNotification) -> String {
    let accent = "#7f8c8d"; // muted slate
    let headline = format!(
        "{}'s record has been archived",
        html_escape(&payload.child_name)
    );
    let preheader = format!(
        "{} has been archived at {}. Their enrollment is no longer active.",
        payload.child_name, payload.school_name
    );

    let details = details_card(&[
        ("Child", payload.child_name.clone()),
        ("School", payload.school_name.clone()),
        ("Status", "Archived".to_string()),
        ("Effective", fmt_datetime(payload.archived_on)),
    ]);

    let body = format!(
        r#"<p>Dear {first_name},</p>
          <p>We're writing to let you know that <strong>{child}</strong>'s record at <strong>{school}</strong> has been archived by your school administrator.</p>
          {details}
          <p style="margin:18px 0 6px 0;"><strong>What this means</strong></p>
          <ul style="margin:0;padding-left:20px;color:#333;">
            <li><strong>{child}</strong>'s enrollment is no longer active</li>
            <li>Forms assigned to <strong>{child}</strong> are no longer required</li>
            <li>Your parent account remains active for any other children enrolled</li>
          </ul>
          <p style="margin-top:18px;">If this was unexpected, or if you have questions about re-enrollment or record retention, please reach out to your school administrator.</p>
          <p style="margin-top:14px;color:#555;">Thank you for being part of the Goddard School community.</p>"#,
        first_name = html_escape(&payload.parent_first_name),
        child = html_escape(&payload.child_name),
        school = html_escape(&payload.school_name),
        details = details,
    );

    render_shell(accent, &headline, &preheader, &body, "")
}

// =====================================================
// Employee lifecycle notification templates
// =====================================================

pub fn employee_invite_html(first_name: &str, last_name: &str, invite_link: &str, school_name: &str) -> String {
    let accent = "#2980b9";
    let headline = format!("Welcome to {} — Employee Access", html_escape(school_name));
    let preheader = format!("You've been added as an employee at {}. Set up your account to get started.", school_name);

    let body = format!(
        r#"<p>Dear {first_name} {last_name},</p>
          <p>You have been added as an employee at <strong>{school}</strong>. To complete your account setup and access your employee dashboard, please click the button below.</p>
          <p style="color:#555;font-size:14px;">This invitation link is valid for 7 days.</p>"#,
        first_name = html_escape(first_name),
        last_name = html_escape(last_name),
        school = html_escape(school_name),
    );

    let cta = cta_button("Set Up Your Account", invite_link, accent);
    render_shell(accent, &headline, &preheader, &body, &cta)
}

pub fn employee_form_assigned_html(employee_name: &str, form_name: &str, due_date: &str, dashboard_url: &str) -> String {
    let accent = "#27ae60";
    let headline = format!("New form assigned: {}", html_escape(form_name));
    let preheader = format!("A new form has been assigned to you. Please complete {} by {}.", form_name, due_date);

    let due = if due_date.is_empty() { "at your earliest convenience".to_string() } else { html_escape(due_date) };

    let details = details_card(&[
        ("Form", form_name.to_string()),
        ("Due Date", due.clone()),
    ]);

    let body = format!(
        r#"<p>Dear {name},</p>
          <p>A new form has been assigned to you that requires your attention.</p>
          {details}
          <p>Please complete this form by <strong>{due}</strong>.</p>"#,
        name = html_escape(employee_name),
        details = details,
        due = due,
    );

    let cta = cta_button("Open Employee Dashboard", dashboard_url, accent);
    render_shell(accent, &headline, &preheader, &body, &cta)
}

pub fn employee_form_approved_html(employee_name: &str, form_name: &str, notes: &str) -> String {
    let accent = "#27ae60";
    let headline = format!("{} has been approved", html_escape(form_name));
    let preheader = format!("Your submission for {} has been approved.", form_name);

    let notes_line = if !notes.is_empty() {
        format!("<p><strong>Notes:</strong> {}</p>", html_escape(notes))
    } else {
        String::new()
    };

    let body = format!(
        r#"<p>Dear {name},</p>
          <p>Your submission for <strong>{form}</strong> has been reviewed and <strong>approved</strong>.</p>
          {notes}
          <p>Thank you for completing this form promptly.</p>"#,
        name = html_escape(employee_name),
        form = html_escape(form_name),
        notes = notes_line,
    );

    render_shell(accent, &headline, &preheader, &body, "")
}

pub fn employee_form_rejected_html(employee_name: &str, form_name: &str, notes: &str) -> String {
    let accent = "#e74c3c";
    let headline = format!("Action needed: {} requires updates", html_escape(form_name));
    let preheader = format!("Your submission for {} has been returned for revision.", form_name);

    let notes_line = if !notes.is_empty() {
        format!("<p><strong>Reviewer notes:</strong> {}</p>", html_escape(notes))
    } else {
        String::new()
    };

    let body = format!(
        r#"<p>Dear {name},</p>
          <p>Your submission for <strong>{form}</strong> has been reviewed and <strong>returned for revision</strong>. Please update and resubmit at your earliest convenience.</p>
          {notes}
          <p>Please log into your employee dashboard to review the details and resubmit.</p>"#,
        name = html_escape(employee_name),
        form = html_escape(form_name),
        notes = notes_line,
    );

    render_shell(accent, &headline, &preheader, &body, "")
}

pub fn employee_form_reminder_html(employee_name: &str, form_name: &str, due_date: &str, dashboard_url: &str) -> String {
    let accent = "#e67e22";
    let headline = format!("Reminder: {} is due soon", html_escape(form_name));
    let preheader = format!("Friendly reminder: please complete {} by {}.", form_name, due_date);

    let due = if due_date.is_empty() { "soon".to_string() } else { html_escape(due_date) };

    let body = format!(
        r#"<p>Dear {name},</p>
          <p>This is a friendly reminder that <strong>{form}</strong> is due <strong>{due}</strong> and has not yet been completed.</p>
          <p>Please log in to your employee dashboard to complete this form.</p>"#,
        name = html_escape(employee_name),
        form = html_escape(form_name),
        due = due,
    );

    let cta = cta_button("Complete Form Now", dashboard_url, accent);
    render_shell(accent, &headline, &preheader, &body, &cta)
}
