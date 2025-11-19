#!/usr/bin/env python3
"""Update README.md with current campaign status from deployed app."""

import json
import re
from datetime import datetime
import sys

def get_previous_dj_count(readme_content):
    """Extract total DJ count from README to detect if list changed."""
    # Look for the DJs column in the table and sum all DJ counts
    pattern = r'\|\s*\[.*?\]\(.*?\)\s*\|\s*(\d+)\s*\|'
    matches = re.findall(pattern, readme_content)
    if matches:
        return sum(int(m) for m in matches)
    return 0

def main():
    try:
        # Read campaign data
        with open('campaigns.json', 'r') as f:
            data = json.load(f)
        
        total_campaigns = data.get('total_campaigns', 0)
        check_interval = data.get('check_interval_minutes', 60)
        campaigns = data.get('campaigns', [])
        
        # Read current README to check if update is needed
        with open('README.md', 'r') as f:
            readme = f.read()
        
        # Calculate current total DJ count
        current_total_dj_count = sum(campaign.get('dj_count', 0) for campaign in campaigns)
        
        # Get previous DJ count from README
        previous_total_dj_count = get_previous_dj_count(readme)
        
        # Check if DJ list has changed
        if current_total_dj_count == previous_total_dj_count and previous_total_dj_count > 0:
            print("No new DJs added - skipping README update")
            return 0
        
        print(f"DJ count changed: {previous_total_dj_count} -> {current_total_dj_count}")
        
        # Generate campaign table
        table_rows = []
        table_rows.append("| Track | DJs |")
        table_rows.append("|-------|-----|")
        
        for campaign in campaigns:
            name = campaign.get('name', 'Unknown')
            url = campaign.get('url', '#')
            track = campaign.get('track_title') or name
            dj_count = campaign.get('dj_count', 0)
            
            # Use track title as link text, fallback to campaign name if no title
            table_rows.append(f"| [{track}]({url}) | {dj_count} |")
        
        # Create status section
        now = datetime.utcnow().strftime('%Y-%m-%d %H:%M UTC')
        status_section = f"""## 🎵 Currently Monitored Campaigns

**Status:** 🟢 Active  
**Total Campaigns:** {total_campaigns}  
**Check Interval:** Every {check_interval} minutes  
**Last Updated:** {now}

{chr(10).join(table_rows)}

> This section is automatically updated when new DJs are added to campaigns.

"""
        
        # Check if section exists and update accordingly
        pattern = r'## 🎵 Currently Monitored Campaigns.*?(?=\n## |\Z)'
        if re.search(pattern, readme, re.DOTALL):
            # Replace existing section
            readme = re.sub(pattern, status_section.rstrip() + '\n', readme, flags=re.DOTALL)
        else:
            # Add after first heading
            lines = readme.split('\n')
            if lines and lines[0].startswith('# '):
                # Insert after first line and any immediate content
                insert_pos = 1
                if len(lines) > 1 and not lines[1].startswith('#'):
                    insert_pos = 2
                lines.insert(insert_pos, '\n' + status_section)
                readme = '\n'.join(lines)
        
        # Write updated README
        with open('README.md', 'w') as f:
            f.write(readme)
        
        print("README updated successfully")
        return 0
    
    except Exception as e:
        print(f"Error updating README: {e}", file=sys.stderr)
        return 1

if __name__ == '__main__':
    sys.exit(main())
