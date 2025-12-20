# FlowTrace User Documentation
## Complete Guide to Financial Transparency

### Table of Contents
1. [Getting Started](#getting-started)
2. [For NGO Directors](#for-ngo-directors)
3. [For Finance Managers](#for-finance-managers)
4. [For SME Owners](#for-sme-owners)
5. [For Donors & Investors](#for-donors--investors)
6. [Mobile App Guide](#mobile-app-guide)
7. [Trust Score Guide](#trust-score-guide)
8. [Integrations](#integrations)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)
11. [FAQs](#faqs)

---

## Getting Started

### Quick Start Guide (5 Minutes)

#### Step 1: Sign Up
1. Visit **app.flowtrace.io**
2. Click "Create Account"
3. Choose your organization type:
   - NGO/Non-profit
   - SME/Business
   - Social Enterprise
4. Enter basic information:
   - Email or phone number
   - Organization name
   - Country (Kenya, Nigeria, Ghana, etc.)
   - Password

#### Step 2: Verify Your Organization
1. Upload registration documents:
   - NGO: Certificate of Registration
   - SME: Business Registration Certificate
2. Add organization logo (optional)
3. Set your primary currency (KES, NGN, GHS, USD)

#### Step 3: Connect Payment Methods
1. Link your mobile money account:
   - **Kenya**: M-Pesa Business Till/Paybill
   - **Nigeria**: MTN MoMo, Airtel Money
   - **Ghana**: MTN Mobile Money, AirtelTigo
2. Add bank account details (if applicable)
3. Enable automatic transaction import

#### Step 4: Invite Your Team
1. Navigate to Settings → Team
2. Add team members by email/phone
3. Assign roles:
   - **Admin**: Full access
   - **Manager**: Create/edit transactions
   - **Viewer**: Read-only access
   - **Auditor**: Special audit access

#### Step 5: Create Your First Transaction
1. Click "Add Transaction" on dashboard
2. Enter amount and select currency
3. Choose source and destination
4. Upload receipt (photo or PDF)
5. Save and watch your Trust Score begin!

---

## For NGO Directors

### Your Daily Workflow

#### Morning Check (2 minutes)
1. **Dashboard Overview**
   ```
   Trust Score: 87 ↗ (+2 from yesterday)
   Pending Verifications: 3
   Weekly Donations: $12,450
   Active Projects: 5
   ```

2. **Quick Actions**
   - Review overnight donations
   - Check flagged transactions
   - Approve pending expenses

#### Weekly Tasks

##### Monday: Donor Report Review
1. Go to Reports → Donor Reports
2. Select previous week's date range
3. Review auto-generated summaries
4. Share public link with major donors

##### Wednesday: Trust Score Optimization
1. Check Trust Score breakdown
2. Address recommendations:
   - "Upload 5 missing receipts"
   - "Verify field office transactions"
   - "Complete monthly reconciliation"

##### Friday: Team Performance
1. Review team activity log
2. Check verification rates by staff
3. Send appreciation messages

### Setting Up for Donor Confidence

#### 1. Customize Your Public Page
**Path**: Settings → Public Profile

```
Your public URL: flowtrace.io/wateraccesskenya
```

**Essential Elements**:
- Organization mission statement
- Current Trust Score badge
- Live donation counter
- Recent project updates
- Financial transparency commitment

**Example Setup**:
```markdown
# Water Access Kenya
**Trust Score: 87** | Verified Organization ✓

## Our Mission
Providing clean water to rural communities across Kenya

## Financial Transparency
- 100% of donations tracked
- Weekly donor reports
- Real-time project updates
- Independent verification

[View Live Financial Flows →]
```

#### 2. Create Compelling Project Pages

**Path**: Projects → New Project

**Example: "Turkana Water Wells Project"**
1. Set project goal: $50,000
2. Define milestones:
   - Phase 1: Survey locations ($5,000)
   - Phase 2: Drill first well ($20,000)
   - Phase 3: Community training ($10,000)
   - Phase 4: Maintenance fund ($15,000)
3. Enable automatic allocation of tagged donations
4. Set up milestone notifications for donors

#### 3. Automate Donor Communications

**Path**: Communications → Automation

**Set up these automatic emails**:
1. **Instant Receipt** (on donation)
   ```
   Subject: Thank you for your $[amount] donation!
   
   Dear [Donor Name],
   
   Your donation has been received and verified.
   Transaction ID: [TXN-ID]
   
   Track your donation: [tracking-link]
   ```

2. **Weekly Impact Update**
   ```
   Subject: Your impact this week at Water Access Kenya
   
   This week, your donations helped:
   - Provide water to 150 families
   - Train 5 community leaders
   - Maintain 3 water points
   
   [View detailed report →]
   ```

3. **Monthly Transparency Report**
   - Full financial summary
   - Trust Score updates
   - Project progress photos
   - Upcoming needs

### Managing Field Operations

#### Setting Up Field Offices
**Path**: Organization → Locations

1. **Add each field office**:
   - Nairobi Headquarters
   - Turkana Field Office
   - Mombasa Operations Center

2. **Assign spending limits**:
   - Petty cash: $500/month
   - Project expenses: $5,000/month
   - Emergency fund: $1,000

3. **Set approval workflows**:
   - Under $100: Auto-approve
   - $100-$1,000: Manager approval
   - Over $1,000: Director approval

#### Field Staff Transaction Recording

**Mobile App Quick Guide for Field Officers**:

1. **Before leaving office**:
   - Sync app (requires internet)
   - Download project details
   - Check camera functionality

2. **In the field**:
   - Record transaction immediately
   - Take photo of receipt
   - Capture GPS location
   - Add beneficiary photo (with permission)
   - Save offline if no network

3. **Back at office**:
   - Connect to WiFi
   - Sync all transactions
   - Review and submit for verification

---

## For Finance Managers

### Daily Financial Management

#### Transaction Processing Workflow

##### Morning Routine (30 minutes)
1. **Import overnight transactions**
   - M-Pesa automatic sync
   - Bank statement import
   - Manual entry review

2. **Categorize new transactions**
   ```
   Drag transactions to categories:
   □ Program Expenses → Water Projects
   □ Operational → Office Rent
   □ Donations → Individual Donors
   □ Grants → USAID Grant 2025
   ```

3. **Verify pending transactions**
   - Match receipts to amounts
   - Confirm GPS locations
   - Approve or flag for review

##### Bulk Transaction Import

**Path**: Transactions → Import

**From M-Pesa**:
1. Download M-Pesa statement (PDF)
2. Click "Import from M-Pesa"
3. Upload PDF file
4. Review parsed transactions
5. Auto-match to existing records
6. Bulk categorize by patterns

**From Bank CSV**:
1. Export bank statement as CSV
2. Map columns:
   ```
   Date → Transaction Date
   Description → Reference
   Debit → Outflow
   Credit → Inflow
   Balance → Running Balance
   ```
3. Apply categorization rules
4. Import and review

### Advanced Categorization

#### Setting Up Smart Rules
**Path**: Settings → Categorization Rules

**Example Rules**:
1. **Salary Payments**
   ```
   IF description contains "salary" OR "payroll"
   AND amount between 20,000-100,000 KES
   THEN category = "Staff Costs"
   ```

2. **Utility Bills**
   ```
   IF sender = "KPLC" OR "Nairobi Water"
   THEN category = "Utilities"
   AND project = "Operations"
   ```

3. **Field Expenses**
   ```
   IF location GPS within "Turkana County"
   AND amount < 50,000 KES
   THEN category = "Field Operations"
   AND require photo verification
   ```

### Month-End Reconciliation

#### Week 4 Checklist
- [ ] All transactions categorized
- [ ] Receipts uploaded (target: 95%+)
- [ ] Bank reconciliation complete
- [ ] Donor restrictions tracked
- [ ] Budget vs. actual reviewed
- [ ] Trust Score optimized

#### Reconciliation Process
1. **Export FlowTrace transactions**
   - Filter by month
   - Export as Excel

2. **Compare with bank statement**
   - Use VLOOKUP to match
   - Identify discrepancies
   - Note timing differences

3. **Adjust for outstanding items**
   - Uncleared checks
   - Deposits in transit
   - Bank charges

4. **Generate reconciliation report**
   - One-click generation
   - Auditor-ready format
   - Digital signature

### Grant Management

#### Setting Up Grant Tracking
**Path**: Projects → Grants

**Example: USAID Water Grant**
1. **Create grant profile**:
   - Grant ID: USAID-2025-001
   - Total amount: $500,000
   - Period: Jan 2025 - Dec 2026
   - Reporting: Quarterly

2. **Define budget categories**:
   ```
   Personnel: $150,000 (30%)
   Equipment: $100,000 (20%)
   Operations: $200,000 (40%)
   Admin: $50,000 (10%)
   ```

3. **Set up restrictions**:
   - No alcohol/entertainment
   - Equipment over $5,000 needs approval
   - 10% indirect cost maximum

4. **Enable automatic allocation**:
   - Tag relevant transactions
   - Auto-calculate burn rate
   - Alert at 80% utilization

#### Donor Reporting

**Generating USAID Report**:
1. Go to Reports → Grant Reports
2. Select "USAID-2025-001"
3. Choose "Standard Federal Format"
4. Review auto-generated sections:
   - Financial summary
   - Budget vs. actual
   - Cost share tracking
   - Program income
5. Export as PDF with attachments

---

## For SME Owners

### Building Investor Confidence

#### Initial Setup for Investment Readiness

1. **Complete Business Profile**
   ```
   AgroTech Solutions Ghana
   Sector: Agriculture Technology
   Founded: 2020
   Employees: 25
   Annual Revenue: $1.2M
   Seeking: $500K Series A
   ```

2. **Historical Data Import**
   - Upload 2 years of financials
   - Import bank statements
   - Add major contracts
   - Document assets

3. **Set Up Investor Dashboard**
   - Real-time P&L
   - Cash flow visualization
   - Burn rate tracking
   - Key metrics display

#### Preparing for Due Diligence

##### Data Room Setup
**Path**: Investor Relations → Data Room

**Organize documents**:
```
📁 Financial Statements
  └── 2023 Audited Financials.pdf
  └── 2024 Management Accounts.xlsx
  └── 2025 YTD Performance.pdf
  
📁 Legal Documents
  └── Incorporation Certificate.pdf
  └── Shareholder Agreement.pdf
  └── IP Assignments.pdf
  
📁 Operations
  └── Major Contracts/
  └── Supplier Agreements/
  └── Employee Handbook.pdf
  
📁 FlowTrace Verified
  └── Trust Score Certificate.pdf
  └── Transaction Verification Report.pdf
  └── 12-Month Flow Analysis.pdf
```

##### Investor Access Management
1. Create investor account
2. Set viewing permissions:
   - Financial data: ✓
   - Operational metrics: ✓
   - Employee details: ✗
   - Supplier contracts: Limited
3. Enable download watermarking
4. Track access logs

### Daily Business Operations

#### Cash Flow Management

**Morning Dashboard Review**:
```
Today's Position:
Opening Balance: $45,670
Expected Inflows: $12,300
Scheduled Outflows: $8,900
Projected Close: $49,070

Alerts:
⚠️ Large payment due in 3 days
✓ Customer invoice paid early
→ Supplier discount available
```

**Weekly Cash Flow Forecast**:
1. Go to Analytics → Cash Flow
2. Review 13-week rolling forecast
3. Identify potential shortfalls
4. Plan payment timing
5. Optimize working capital

#### Customer & Supplier Management

**Setting Up Payment Terms**:
1. **Customer Categories**:
   ```
   Enterprise: NET 30
   SME: NET 15
   Retail: Immediate
   Government: NET 45
   ```

2. **Automated Invoicing**:
   - Create invoice templates
   - Set recurring invoices
   - Enable payment tracking
   - Send automatic reminders

3. **Supplier Payment Optimization**:
   - Track early payment discounts
   - Monitor payment terms
   - Set up approval workflows
   - Batch payment processing

---

## For Donors & Investors

### Discovering Trustworthy Organizations

#### Using the Search Function
**Path**: discover.flowtrace.io

**Filter Options**:
- Country: Kenya, Nigeria, Ghana
- Sector: Water, Health, Education
- Trust Score: 80+ (High Trust)
- Size: Budget range
- Impact: Beneficiaries served

**Understanding Search Results**:
```
Water Access Kenya
Trust Score: 87 | ✓ Verified
Sector: Water & Sanitation
Annual Budget: $2.4M
Beneficiaries: 45,000+
[View Profile →]
```

### Evaluating an Organization

#### Trust Score Deep Dive

**Understanding the Score**:
```
Overall Score: 87/100

Breakdown:
├── Transaction Verification: 92%
│   └── 95% of transactions have receipts
├── Documentation: 85%
│   └── All required documents uploaded
└── Consistency: 82%
    └── Spending matches stated programs
```

**Red Flags to Watch**:
- Trust Score below 60
- Declining score trend
- High % unverified transactions
- Mismatched spending patterns
- No recent updates

#### Analyzing Financial Flows

**Interactive Flow Diagram**:
1. Click on any node to see details
2. Follow money from source to end use
3. Check timeline view for patterns
4. Look for:
   - Clear program allocation
   - Reasonable admin costs (10-20%)
   - Consistent spending patterns
   - No suspicious diversions

**Example Analysis**:
```
USAID Grant ($100,000)
    ├── Programs (70%)
    │   ├── Water Infrastructure: $50,000
    │   └── Community Training: $20,000
    ├── Operations (20%)
    │   ├── Staff Salaries: $15,000
    │   └── Office Costs: $5,000
    └── Admin (10%)
        └── Audit & Compliance: $10,000
```

### Making a Donation

#### One-Time Donation Process
1. Click "Donate" on organization page
2. Select amount and currency
3. Choose payment method:
   - Credit/Debit card
   - Bank transfer
   - Mobile money
   - Cryptocurrency (select orgs)
4. Designate funds (optional):
   - General support
   - Specific project
   - Where needed most
5. Receive instant confirmation

#### Setting Up Recurring Donations
1. Choose frequency:
   - Monthly
   - Quarterly
   - Annually
2. Set amount and start date
3. Enable impact notifications
4. Review and confirm
5. Manage via donor dashboard

### Tracking Your Impact

#### Donor Dashboard Features
**Path**: my.flowtrace.io

**Your Impact Summary**:
```
Total Donated: $5,450
Organizations Supported: 3
Lives Impacted: 1,200+
Trust Score Average: 85

Recent Activity:
✓ $500 to Water Access Kenya
  └── Funded 2 water pumps
✓ $200 to EduTech Nigeria  
  └── Provided 50 tablets
```

#### Real-Time Tracking
1. **Transaction Timeline**:
   - See when donation received
   - Track through organization
   - View field verification
   - See beneficiary impact

2. **Project Updates**:
   - Photo evidence
   - GPS coordinates
   - Progress milestones
   - Completion certificates

3. **Communication Preferences**:
   - Instant notifications
   - Weekly summaries
   - Monthly reports
   - Annual tax documents

---

## Mobile App Guide

### Installation

#### Android (Most Common)
1. Open Google Play Store
2. Search "FlowTrace"
3. Download app (12MB)
4. Works on Android 5.0+
5. Supports offline mode

#### iOS
1. Open App Store
2. Search "FlowTrace"
3. Download app
4. Works on iOS 11+
5. Enable notifications

#### Progressive Web App (No Download)
1. Visit app.flowtrace.io on mobile
2. Click "Add to Home Screen"
3. Works like native app
4. Updates automatically
5. Uses less storage

### Offline Functionality

#### Before Going Offline
1. Open app while connected
2. Go to Settings → Offline Mode
3. Download:
   - Organization data
   - Recent transactions
   - Project details
   - Exchange rates
4. Verify download complete

#### Working Offline
**What You CAN Do**:
- Record new transactions
- Take receipt photos
- Capture GPS coordinates
- Add notes and tags
- View recent data

**What You CAN'T Do**:
- Sync with bank accounts
- View Trust Score updates
- Generate new reports
- Invite team members
- Process payments

#### Syncing When Back Online
1. Connect to internet
2. App auto-detects connection
3. See sync status:
   ```
   Syncing...
   ↑ 15 transactions
   ↑ 23 photos
   ↓ 3 updates
   ✓ Sync complete
   ```
4. Resolve any conflicts
5. Verify all uploaded

### Mobile-Specific Features

#### Quick Transaction Entry
1. **Shake to Add**: Shake phone to quickly add transaction
2. **Voice Input**: 
   - "Add expense fifty thousand shillings for water pumps"
   - "Received donation two hundred dollars from John Smith"
3. **SMS Integration**: Forward M-Pesa SMS for auto-entry

#### Location Services
- **Auto GPS**: Captures coordinates with each transaction
- **Geofencing**: Reminds to record transactions at project sites
- **Map View**: See all transactions on map

#### Camera Features
- **Multi-Receipt**: Capture multiple receipts in sequence
- **OCR Processing**: Extracts amount and date automatically
- **Low-Light Mode**: Enhanced capture in poor lighting
- **Batch Upload**: Queue photos for later upload

### Data Usage Optimization

#### Minimizing Data Costs
1. **Settings → Data Saver**:
   - Compress images before upload
   - Sync only on WiFi
   - Reduce background updates
   - Text-only mode available

2. **Efficient Syncing**:
   - Batch uploads during off-peak
   - Use office WiFi for reports
   - Download reports as needed
   - Clear old cached data

---

## Trust Score Guide

### Understanding Your Score

#### Score Ranges
- **90-100**: Exceptional Transparency
- **80-89**: Excellent Transparency  
- **70-79**: Good Transparency
- **60-69**: Adequate Transparency
- **Below 60**: Needs Improvement

#### Calculation Factors

##### 1. Transaction Verification (40% weight)
**How to Maximize**:
- Upload receipts for 95%+ transactions
- Add GPS coordinates
- Include photos when relevant
- Get third-party verification
- Respond to queries quickly

**Point Breakdown**:
```
Receipt uploaded: +2 points
GPS captured: +1 point
Photo evidence: +1 point
Third-party verified: +2 points
Query resolved <24hrs: +1 point
```

##### 2. Documentation Completeness (30% weight)
**Required Documents**:
- ✓ Registration certificate
- ✓ Annual reports (2 years)
- ✓ Audit reports
- ✓ Board resolutions
- ✓ Tax compliance

**Bonus Documents**:
- Impact assessments
- Beneficiary testimonials
- Partner references
- Media coverage
- Awards/recognition

##### 3. Financial Consistency (30% weight)
**Measured By**:
- Spending matches budget
- Regular transaction patterns
- No unexplained spikes
- Proper categorization
- Timely reconciliation

### Improving Your Score

#### Quick Wins (Can improve 5-10 points)
1. **Upload Missing Receipts**
   - Go to Transactions → Unverified
   - Bulk upload receipts
   - Add explanations for missing ones

2. **Complete Reconciliation**
   - Match bank statements
   - Explain discrepancies
   - Mark period as reconciled

3. **Update Documentation**
   - Upload latest annual report
   - Add board minutes
   - Include audit reports

#### Long-Term Improvements
1. **Build Verification Habits**:
   - Train all staff on requirements
   - Make receipt collection mandatory
   - Use mobile app in field
   - Regular verification reviews

2. **Enhance Transparency**:
   - Publish monthly updates
   - Share impact stories
   - Respond to donor queries
   - Host transparency webinars

3. **System Integration**:
   - Connect bank accounts
   - Automate categorization
   - Set up approval workflows
   - Enable real-time sync

### Trust Score Certification

#### Getting Certified
1. Maintain score 80+ for 6 months
2. Pass verification audit
3. Complete transparency pledge
4. Receive certificate and badge

#### Using Certification
- Add to proposals
- Display on website
- Include in email signatures
- Share on social media
- Reference in pitches

---

## Integrations

### Payment Systems

#### M-Pesa Integration (Kenya)
**Setup Process**:
1. Go to Integrations → M-Pesa
2. Enter Paybill/Till number
3. Provide API credentials:
   - Consumer Key
   - Consumer Secret
   - Passkey
4. Test connection
5. Enable auto-import

**Features Enabled**:
- Real-time transaction sync
- Automatic categorization
- SMS backup system
- Bulk payment processing
- Settlement reconciliation

#### Bank Integrations

**Supported Banks**:
- **Kenya**: KCB, Equity, Co-op Bank
- **Nigeria**: GTBank, First Bank, Access
- **Ghana**: GCB, Ecobank, Stanbic

**Connection Methods**:
1. **API Direct** (Best):
   - Real-time sync
   - Two-way communication
   - Automatic reconciliation

2. **Statement Upload**:
   - Daily PDF/CSV upload
   - Semi-automated matching
   - Manual verification

### Accounting Software

#### QuickBooks Integration
1. **Export from FlowTrace**:
   - Select date range
   - Choose export format
   - Map categories to QB accounts
   - Download file

2. **Import to QuickBooks**:
   - Use IIF format
   - Review mappings
   - Verify balances
   - Complete import

#### Xero Integration
- Direct API connection
- Two-way sync available
- Automatic journal entries
- Invoice matching

### Communication Tools

#### WhatsApp Business
**Automated Messages**:
- Transaction confirmations
- Receipt reminders
- Report delivery
- Donor updates

**Setup**:
1. Connect WhatsApp Business account
2. Create message templates
3. Set trigger rules
4. Test and activate

#### Email Integration
- Gmail/Outlook sync
- Automated reports
- Donor communications
- Team notifications

---

## Best Practices

### For Maximum Trust

#### The 24-Hour Rule
**Commit to recording transactions within 24 hours**
- Memories are fresh
- Receipts aren't lost
- Patterns are consistent
- Trust Score stays high

#### The Verification Triad
**Every transaction should have**:
1. **Documentary proof** (receipt/invoice)
2. **Contextual proof** (photo/description)
3. **Location proof** (GPS/address)

#### Transparency Calendar
**Monday**: Review weekend transactions
**Tuesday**: Send donor updates
**Wednesday**: Team verification check
**Thursday**: Categorization review
**Friday**: Weekly report generation

### For Efficient Operations

#### Bulk Processing Tips
1. **Set aside time blocks**:
   - Morning: Transaction entry
   - Afternoon: Verification
   - Evening: Report review

2. **Use keyboard shortcuts**:
   - `Ctrl+N`: New transaction
   - `Ctrl+U`: Upload receipt
   - `Ctrl+S`: Save and next
   - `Ctrl+R`: Generate report

3. **Create templates**:
   - Recurring expenses
   - Standard donations
   - Regular transfers
   - Common categories

#### Team Coordination
1. **Clear role definition**:
   ```
   Field Officers: Record & photograph
   Finance Team: Verify & categorize
   Director: Approve & report
   ```

2. **Daily standups**:
   - Pending verifications
   - Unusual transactions
   - System issues
   - Training needs

3. **Monthly reviews**:
   - Trust Score analysis
   - Process improvements
   - Staff recognition
   - Goal setting

### Security Best Practices

#### Account Security
1. **Strong passwords**: 12+ characters with mix
2. **Two-factor authentication**: Always enabled
3. **Regular password changes**: Every 90 days
4. **Unique passwords**: Never reuse
5. **Password manager**: Recommended

#### Data Protection
1. **Limit access**: Only necessary personnel
2. **Regular audits**: Check who accessed what
3. **Secure devices**: Lock screens, encryption
4. **Safe networks**: Avoid public WiFi
5. **Backup codes**: Store securely offline

#### Financial Controls
1. **Dual approval**: For large transactions
2. **Segregation**: Entry vs approval
3. **Regular reviews**: Daily anomalies
4. **Audit trails**: Never delete, only void
5. **Reconciliation**: Monthly minimum

---

## Troubleshooting

### Common Issues

#### "Cannot sync transactions"
**Solutions**:
1. Check internet connection
2. Verify API credentials
3. Look for service alerts
4. Try manual refresh
5. Contact support if persists

#### "Receipt upload failing"
**Check**:
- File size (max 10MB)
- File format (JPG, PNG, PDF)
- Internet speed
- Storage space
- Camera permissions

#### "Trust Score not updating"
**Verify**:
- Recent transactions verified
- Documents up to date
- No pending flags
- Reconciliation complete
- 24-hour processing time

#### "Cannot login"
**Try**:
1. Password reset
2. Clear browser cache
3. Try different browser
4. Check account status
5. Contact admin

### Mobile App Issues

#### "App keeps crashing"
1. Update to latest version
2. Clear app cache
3. Free up phone storage
4. Restart phone
5. Reinstall if needed

#### "Offline sync not working"
1. Check sync settings
2. Ensure WiFi connected
3. Manual sync trigger
4. Check conflict queue
5. Reset offline data

### Integration Problems

#### "Bank feed disconnected"
1. Re-authenticate connection
2. Check bank's status
3. Update API credentials
4. Verify IP whitelist
5. Contact bank support

#### "M-Pesa not importing"
1. Verify API credentials
2. Check callback URL
3. Test with single transaction
4. Review error logs
5. Check Safaricom status

---

## FAQs

### General Questions

**Q: How much does FlowTrace cost?**
A: Pricing tiers:
- Small NGO (<$100K budget): $99/month
- Medium NGO: $299/month
- Large NGO/SME: $499/month
- Enterprise: Custom pricing
- 20% discount for annual payment

**Q: Is my data secure?**
A: Yes, we use:
- Bank-level encryption (AES-256)
- Secure data centers
- Regular security audits
- GDPR compliance
- Local data protection laws

**Q: Can I export my data?**
A: Yes, anytime:
- Full data export in CSV/Excel
- PDF reports
- API access (Pro plans)
- No vendor lock-in

**Q: What happens if I go over limits?**
A: Flexible upgrades:
- Notification at 80% usage
- Automatic upgrade option
- No service interruption
- Pay-as-you-go available

### Trust Score Questions

**Q: How often is Trust Score updated?**
A: Real-time with 24-hour finalization:
- Instant updates for verifications
- Daily score recalculation
- Weekly trend analysis
- Monthly peer comparison

**Q: Can Trust Score go down?**
A: Yes, it can decrease if:
- Verification rate drops
- Documents expire
- Anomalies detected
- Long gaps in updates
- Reconciliation issues

**Q: Is Trust Score public?**
A: You control visibility:
- Public badge optional
- Detailed breakdown private
- Share via secure link
- API access controlled

### Technical Questions

**Q: Which browsers work best?**
A: Modern browsers:
- Chrome (recommended)
- Firefox
- Safari
- Edge
- Mobile browsers

**Q: How much bandwidth needed?**
A: Minimal requirements:
- Basic use: 2G/3G sufficient
- Reports: 4G recommended
- Video verification: WiFi preferred
- Offline mode available

**Q: Can I integrate with my website?**
A: Yes, multiple options:
- Trust Score widget
- Donation widget
- Full iframe embed
- API integration
- WordPress plugin

### Support Questions

**Q: How do I get help?**
A: Multiple channels:
- In-app chat (fastest)
- Email: support@flowtrace.io
- WhatsApp: +254-XXX-XXXX
- Help center: help.flowtrace.io
- Video tutorials

**Q: What are support hours?**
A: Based on your region:
- Kenya: 8 AM - 8 PM EAT
- Nigeria: 8 AM - 8 PM WAT
- Emergency: 24/7 for critical issues
- Response time: <2 hours (business)

**Q: Is training available?**
A: Yes, we offer:
- Free onboarding session
- Weekly webinars
- Video library
- In-person training (Enterprise)
- Certification program

---

## Conclusion

FlowTrace transforms financial management from a compliance burden into a competitive advantage. By following this guide, you'll build trust, attract funding, and create lasting impact.

**Remember**: Transparency isn't just about technology—it's about commitment to openness, consistency in practice, and continuous improvement.

**Need more help?** 
- Visit: help.flowtrace.io
- Email: success@flowtrace.io
- Call/WhatsApp: +254-XXX-XXXX

*Building financial trust, one transaction at a time.*

---

**Version**: 1.0  
**Last Updated**: June 2025  
**Next Review**: September 2025

© 2025 FlowTrace. Empowering transparency across Africa.