#!/usr/bin/env python3
"""Update README.md with current campaign status from deployed app."""

import json
import re
from datetime import datetime
import sys

def main():
    try:
        # Read campaign data
        with open('campaigns.json', 'r') as f:
            data = json.load(f)
        
        total_campaigns = data.get('total_campaigns', 0)
        check_interval = data.get('check_interval_minutes', 60)
        campaigns = data.get('campaigns', [])
        
        # Generate campaign table
        table_rows = []
        table_rows.append("| Campaign | Track | DJs | Last Checked |")
        table_rows.append("|----------|-------|-----|--------------|")
        
        for campaign in campaigns:
            name = campaign.get('name', 'Unknown')
            url = campaign.get('url', '#')
            track = campaign.get('track_title') or name
            dj_count = campaign.get('dj_count', 0)
            last_checked = campaign.get('last_checked', 'N/A')
            
            # Format timestamp
            if last_checked and last_checked != 'N/A':
                try:
                    dt = datetime.fromisoformat(last_checked.replace('Z', '+00:00'))
                    last_checked = dt.strftime('%Y-%m-%d %H:%M')
                except:
                    pass
            
            table_rows.append(f"| [{name}]({url}) | {track} | {dj_count} | {last_checked} |")
        
        # Create status section
        now = datetime.utcnow().strftime('%Y-%m-%d %H:%M UTC')
        status_section = f"""## 🎵 Currently Monitored Campaigns

**Status:** 🟢 Active  
**Total Campaigns:** {total_campaigns}  
**Check Interval:** Every {check_interval} minutes  
**Last Updated:** {now}

{chr(10).join(table_rows)}

> This section is automatically updated every 6 hours by querying the deployed application.

"""
        
        # Read README
        with open('README.md', 'r') as f:
            readme = f.read()
        
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
