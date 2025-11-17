# Mailgun Setup Guide

This guide will help you set up Mailgun for sending email notifications from the Inflyte DJ Monitor.

## Overview

Mailgun is an email delivery service that provides:
* **Free Tier**: 5, 000 emails per month (more than enough for DJ alerts)
* **Reliable Delivery**: High deliverability rates
* **Simple API**: Easy integration with REST API

## Step 1: Create a Mailgun Account

1. Go to https://signup.mailgun.com/new/signup
2. Sign up for a free account
3. Verify your email address
4. Complete the account setup

## Step 2: Get Your API Key

1. Log in to https://app.mailgun.com
2. Click on your account name (top right)
3. Select **API Keys** from the dropdown
4. Copy your **Private API key** (starts with `key-...`)

**Important:** Keep this key secret! Never commit it to version control.

## Step 3: Choose a Domain

You have two options:

### Option A: Use Sandbox Domain (Quick Start)

Mailgun provides a sandbox domain for testing:

1. Go to **Sending** → **Domains** in the Mailgun dashboard
2. You'll see a sandbox domain like `sandbox1234567890abcdef.mailgun.org`
3. Click on it to see details

**Limitations:**
* Can only send to **authorized recipients** (up to 5)
* Emails will have a note saying they're from a sandbox

### Option B: Add Your Own Domain (Recommended for Production)

1. Go to **Sending** → **Domains**
2. Click **Add New Domain**
3. Enter your domain (e.g., `mg.yourdomain.com`)
4. Follow the DNS setup instructions:
   - Add TXT records for domain verification
   - Add MX records for receiving emails
   - Add CNAME records for tracking

**Verification takes 24-48 hours** but you can use sandbox in the meantime.

## Step 4: Authorize Recipients (Sandbox Only)

If using a sandbox domain, you must authorize recipients:

1. Go to **Sending** → **Domains**
2. Click on your sandbox domain
3. Scroll to **Authorized Recipients**
4. Click **Add Recipient**
5. Enter the email address that will receive alerts
6. Check your email and click the verification link

**Repeat for any additional recipients.**

## Step 5: Configure Environment Variables

Add to your `.env` file:

```bash
# Mailgun Configuration
MAILGUN_API_KEY=key-1234567890abcdef1234567890abcdef
MAILGUN_DOMAIN=sandbox1234567890abcdef.mailgun.org
RECIPIENT_EMAIL=your-email@example.com
FROM_EMAIL=noreply@sandbox1234567890abcdef.mailgun.org

# For verified domain:
# MAILGUN_DOMAIN=mg.yourdomain.com
# FROM_EMAIL=alerts@mg.yourdomain.com
```

**Values to replace:**
* `MAILGUN_API_KEY`: Your private API key from Step 2
* `MAILGUN_DOMAIN`: Your sandbox or verified domain
* `RECIPIENT_EMAIL`: Where you want to receive alerts
* `FROM_EMAIL`: Sender address (must use your Mailgun domain)

## Step 6: Test Email Sending

Run the DJ monitor and verify it can send emails:

```bash
# Set up a test by removing some DJs from the stored list
# This will trigger a "new DJs detected" alert

cargo run
```

When new DJs are detected, you should see:

```
✅ Email notification sent to your-email@example.com
```

Check your email inbox for the alert!

## Understanding Mailgun Regions

Mailgun has two regions:

### US Region (Default)

* API endpoint: `https://api.mailgun.net`
* Most common, used by default in the app

### EU Region

* API endpoint: `https://api.eu.mailgun.net`
* For GDPR compliance

**If you're using EU region**, you'll need to modify the code to use the EU endpoint.

## Email Format

The app sends two versions of each email:

### HTML Version (Rich)

* Styled with colors and formatting
* Professional appearance
* Gradient header
* Formatted DJ list

### Text Version (Fallback)

* Plain text for email clients that don't support HTML
* Same information, simpler format

## Step 7: Verify Email Delivery

### Check Mailgun Logs

1. Go to **Sending** → **Logs** in Mailgun dashboard
2. You should see your sent email
3. Check the status: `delivered`, `opened`, etc.

### Check Your Inbox

1. Look for email with subject like: `🚨 2 New DJs Added to Inflyte Support List`
2. If not in inbox, check spam/junk folder
3. Mark as "Not Spam" if needed

### Common Issues

**Email in spam folder:**
* Add sender to contacts
* Mark as "Not Spam"
* For production, verify your domain and set up SPF/DKIM

**Email not received:**
* Check Mailgun logs for errors
* Verify recipient is authorized (for sandbox)
* Check recipient email address is correct

## Upgrading from Sandbox to Verified Domain

Once you're ready for production:

### 1. Verify Your Domain

Follow the DNS setup in Mailgun dashboard:

```bash
# Example DNS records (use your actual values from Mailgun)
mg.yourdomain.com    TXT    "v=spf1 include:mailgun.org ~all"
mg.yourdomain.com    TXT    "k=rsa; p=MIG..."  # DKIM key
mx._domainkey.mg.yourdomain.com    CNAME    mailgun.org
```

### 2. Update Environment Variables

```bash
MAILGUN_DOMAIN=mg.yourdomain.com
FROM_EMAIL=alerts@mg.yourdomain.com
```

### 3. Benefits

* No recipient authorization needed
* Better deliverability
* Custom from address
* Professional appearance
* No sandbox warnings

## Email Customization

The email template is in `src/main.rs` in the `send_email_alert()` function.

You can customize:
* **Subject line** - Format of the alert subject
* **HTML styling** - Colors, fonts, layout
* **Content** - What information is included
* **From name** - Change `FROM_EMAIL` or modify code

Example customization:

```rust
// In src/main.rs, modify the email HTML
let html_body = format!(
    r#"<!DOCTYPE html>
    <html>
    <head>
        <style>
            /* Your custom CSS here */
            .header {{ background: #FF6B6B; }}  /* Change color */
        </style>
    </head>
    ...
```

## Rate Limits

### Free Tier Limits

* **5, 000 emails/month** - More than enough for DJ alerts
* No daily sending limit
* No credit card required

### Typical Usage

* 1-2 alerts per week = ~8-10 emails/month
* Well within free tier limits

## Monitoring Email Usage

Check your email usage:

1. Go to **Account** → **Plan Details**
2. View **Emails sent this month**
3. See remaining emails in free tier

## Troubleshooting

### "Failed to send email via Mailgun"

**Check API key:**

```bash
# Test with curl
curl -s --user 'api:YOUR-API-KEY' \
  https://api.mailgun.net/v3/YOUR-DOMAIN/messages \
  -F from='test@YOUR-DOMAIN' \
  -F to='you@example.com' \
  -F subject='Test' \
  -F text='Testing'
```

### "Forbidden: Sandbox domain only allows authorized recipients"

* Recipient must be authorized in Mailgun dashboard
* Check **Sending** → **Domains** → **Authorized Recipients**
* Add and verify the recipient email

### "Domain not found"

* Check `MAILGUN_DOMAIN` in `.env`
* It should match exactly what's in Mailgun dashboard
* Include `.mailgun.org` for sandbox domains

### "Unauthorized: Invalid API key"

* Verify API key in Mailgun dashboard
* Use **Private API key**, not public key
* Check for extra spaces in `.env` file

### Email shows as "sent" but not received

* Check spam/junk folder
* Verify recipient email address is correct
* Check Mailgun logs for delivery status
* For sandbox, ensure recipient is authorized

## Best Practices

### Security

* Never commit `.env` file to git (it's in `.gitignore`)
* Keep API keys secret
* Rotate API keys periodically
* Use environment variables, not hardcoded values

### Deliverability

* Verify your domain for better deliverability
* Set up SPF and DKIM records
* Avoid trigger words in subject lines
* Keep email content relevant

### Testing

* Test with sandbox before going to production
* Send test emails before deployment
* Monitor Mailgun logs regularly
* Check spam folder initially

## Alternative Email Services

If you prefer a different service, you can modify `send_email_alert()` in `src/main.rs` to use:

* **SendGrid** - Similar API, free tier available
* **Amazon SES** - AWS service, pay-as-you-go
* **SMTP** - Use any SMTP server (Gmail, etc.)

## Cost Comparison

| Service | Free Tier | After Free Tier |
|---------|-----------|-----------------|
| Mailgun | 5, 000/month | $35/50k emails |
| SendGrid | 100/day | $19.95/50k emails |
| AWS SES | 62, 000/month* | $0.10/1k emails |
| Gmail SMTP | ~500/day** | N/A |

*When sending from EC2
**Unofficial limit, subject to change

## Next Steps

1. ✅ Create Mailgun account
2. ✅ Get API key
3. ✅ Configure domain (sandbox or verified)
4. ✅ Authorize recipients (if using sandbox)
5. ✅ Update `.env` file
6. ✅ Test email sending
7. ⏭️ Run the monitor!

## Additional Resources

* [Mailgun Documentation](https://documentation.mailgun.com/)
* [Mailgun API Reference](https://documentation.mailgun.com/en/latest/api_reference.html)
* [Domain Verification Guide](https://help.mailgun.com/hc/en-us/articles/360026833053)
