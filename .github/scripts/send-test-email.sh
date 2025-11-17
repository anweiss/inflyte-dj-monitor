#!/bin/bash
set -e

# Test email notification script for Mailgun
# This script sends test emails to verify the Mailgun integration

TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

if [ "$TEST_TYPE" = "new_djs_detected" ]; then
  SUBJECT="🚨 TEST: New DJs Detected on Inflyte Support List"
  
  HTML_CONTENT="<!DOCTYPE html><html><head><style>body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif;line-height:1.6;color:#333;max-width:600px;margin:0 auto;padding:20px}.header{background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);color:white;padding:30px;border-radius:10px 10px 0 0;text-align:center}.content{background:#f8f9fa;padding:30px;border-radius:0 0 10px 10px}.dj-list{background:white;border-left:4px solid #667eea;padding:15px;margin:20px 0;border-radius:5px}.dj-item{padding:10px 0;border-bottom:1px solid #e9ecef}.dj-item:last-child{border-bottom:none}.alert-badge{background:#dc3545;color:white;padding:5px 15px;border-radius:20px;font-size:14px;display:inline-block;margin-bottom:10px}.footer{margin-top:20px;padding-top:20px;border-top:1px solid #dee2e6;font-size:12px;color:#6c757d}.test-banner{background:#ffc107;color:#000;padding:15px;border-radius:5px;margin-bottom:20px;text-align:center;font-weight:bold}</style></head><body><div class='header'><h1>🎧 Inflyte DJ Monitor Alert</h1><p>Support List Update Detected</p></div><div class='content'><div class='test-banner'>⚠️ THIS IS A TEST EMAIL - No actual changes detected</div><div class='alert-badge'>TEST MODE</div><p><strong>Sample Alert:</strong> The following DJs would have been detected as new additions:</p><div class='dj-list'><div class='dj-item'>🎵 DJ Test One</div><div class='dj-item'>🎵 DJ Test Two</div><div class='dj-item'>🎵 DJ Test Three</div></div><p><strong>Timestamp:</strong> ${TIMESTAMP}</p><p><strong>Triggered by:</strong> GitHub Actions Manual Workflow</p><p>This is a test email to verify that the Mailgun email notification system is working correctly.</p><div class='footer'><p>This email was sent by the Inflyte DJ Monitor testing system.</p><p>To stop receiving these notifications, update your GitHub secrets configuration.</p></div></div></body></html>"
  
  TEXT_CONTENT="INFLYTE DJ MONITOR - TEST ALERT

⚠️ THIS IS A TEST EMAIL - No actual changes detected

Sample Alert: The following DJs would have been detected as new additions:

- DJ Test One
- DJ Test Two  
- DJ Test Three

Timestamp: ${TIMESTAMP}
Triggered by: GitHub Actions Manual Workflow

This is a test email to verify that the Mailgun email notification system is working correctly.

---
This email was sent by the Inflyte DJ Monitor testing system."

else
  SUBJECT="${CUSTOM_SUBJECT:-🧪 TEST: Custom Email from Inflyte Monitor}"
  CUSTOM_MSG="${CUSTOM_MESSAGE:-This is a custom test email from the Inflyte DJ Monitor system.}"
  
  HTML_CONTENT="<!DOCTYPE html><html><head><style>body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif;line-height:1.6;color:#333;max-width:600px;margin:0 auto;padding:20px}.header{background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);color:white;padding:30px;border-radius:10px 10px 0 0;text-align:center}.content{background:#f8f9fa;padding:30px;border-radius:0 0 10px 10px}.test-banner{background:#17a2b8;color:white;padding:15px;border-radius:5px;margin-bottom:20px;text-align:center;font-weight:bold}.message-box{background:white;border-left:4px solid #17a2b8;padding:20px;margin:20px 0;border-radius:5px}.footer{margin-top:20px;padding-top:20px;border-top:1px solid #dee2e6;font-size:12px;color:#6c757d}</style></head><body><div class='header'><h1>🎧 Inflyte DJ Monitor</h1><p>Custom Test Message</p></div><div class='content'><div class='test-banner'>🧪 CUSTOM TEST EMAIL</div><div class='message-box'><p>${CUSTOM_MSG}</p></div><p><strong>Timestamp:</strong> ${TIMESTAMP}</p><p><strong>Triggered by:</strong> GitHub Actions Manual Workflow</p><div class='footer'><p>This email was sent by the Inflyte DJ Monitor testing system.</p></div></div></body></html>"
  
  TEXT_CONTENT="INFLYTE DJ MONITOR - CUSTOM TEST EMAIL

${CUSTOM_MSG}

Timestamp: ${TIMESTAMP}
Triggered by: GitHub Actions Manual Workflow

---
This email was sent by the Inflyte DJ Monitor testing system."
fi

echo "Sending test email to: $RECIPIENT_EMAIL"
echo "Subject: $SUBJECT"

# Create temporary files for content to avoid curl issues with long strings
TEXT_FILE=$(mktemp)
HTML_FILE=$(mktemp)
echo "$TEXT_CONTENT" > "$TEXT_FILE"
echo "$HTML_CONTENT" > "$HTML_FILE"

RESPONSE=$(curl -s -w "\n%{http_code}" --user "api:$MAILGUN_API_KEY" \
    "https://api.mailgun.net/v3/$MAILGUN_DOMAIN/messages" \
    -F "from=Inflyte Monitor <$FROM_EMAIL>" \
    -F "to=$RECIPIENT_EMAIL" \
    -F "subject=$SUBJECT" \
    -F "text=<$TEXT_FILE" \
    -F "html=<$HTML_FILE")

# Clean up temp files
rm -f "$TEXT_FILE" "$HTML_FILE"

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

echo "Response status: $HTTP_CODE"
echo "Response body: $BODY"

if [ "$HTTP_CODE" -ge 200 ] && [ "$HTTP_CODE" -lt 300 ]; then
  echo "✅ Test email sent successfully!"
  MSG_ID=$(echo "$BODY" | grep -o '"id":"[^"]*"' | head -n1 | cut -d'"' -f4)
  echo "Message ID: $MSG_ID"
else
  echo "❌ Failed to send test email"
  echo "Error: $BODY"
  exit 1
fi
