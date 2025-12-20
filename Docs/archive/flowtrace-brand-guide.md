# FlowTrace Brand Guidelines
## Version 1.0 | June 2025

---

## Table of Contents
1. [Brand Philosophy](#brand-philosophy)
2. [Logo System](#logo-system)
3. [Typography](#typography)
4. [Color System](#color-system)
5. [Visual Design System](#visual-design-system)
6. [Iconography](#iconography)
7. [Motion Principles](#motion-principles)
8. [Component Library](#component-library)
9. [Brand Applications](#brand-applications)
10. [Digital Guidelines](#digital-guidelines)
11. [Do's and Don'ts](#dos-and-donts)

---

## Brand Philosophy

### Mission
To transform financial flows into visual trust, enabling African organizations to unlock global investment through radical transparency.

### Brand Essence
**"Invisible Excellence"** - The most sophisticated technology disappears into intuitive experience.

### Design Principles

#### 1. Restraint as Luxury
Less is exponentially more. Every element must earn its place through function and beauty.

#### 2. Data as Art
Financial flows should be as beautiful as they are informative. Complexity made elegant.

#### 3. Respectful Minimalism
Clean but never cold. Minimal but never empty. Professional but never corporate.

#### 4. Trust Through Clarity
Every pixel should reinforce transparency. No dark patterns, no confusion, no ambiguity.

### Brand Personality
- **Precise** - Swiss-watch accuracy in every interaction
- **Trustworthy** - Bank-vault security with open-door transparency  
- **Progressive** - Leading African fintech into the future
- **Human** - Technology that serves people, not the other way around

---

## Logo System

### Primary Logo Mark

#### The Trust Node
A continuous line forming three connected nodes, creating both an abstract 'F' and a flow network visualization.

```
Construction Grid (16x16)

    2   4   6   8   10  12  14  16
2   ●━━━━━━━━━━━━━━━●
4   ┃               
6   ┃               
8   ●━━━━━━━━━━━━━━━●
10  ┃               
12  ┃               
14  ●               
```

#### Logo Specifications
- **Minimum Size**: 24px height (digital), 8mm height (print)
- **Clear Space**: 2x the circle diameter on all sides
- **Line Weight**: 2px at base size (scales proportionally)
- **Corner Radius**: Perfect circles for nodes

### Logo Variations

#### Horizontal Lockup
```
○━━○━━○  FlowTrace
```

#### Stacked Lockup
```
  ○━━○━━○
 FlowTrace
```

#### Icon Only
```
○━━○━━○
```

### Logo Colors
- **Primary**: #0A0F1C (Trust Blue) on light backgrounds
- **Reversed**: #FFFFFF on dark backgrounds
- **Never**: Gradients, effects, or multiple colors

### Logo Don'ts
- ❌ Don't rotate or skew
- ❌ Don't change proportions
- ❌ Don't add effects or shadows
- ❌ Don't use on busy backgrounds
- ❌ Don't recreate or trace

---

## Typography

### Type Scale
Based on a 1.250 ratio (Major Third) for harmonious scaling.

```
Display:    48px / 56px line-height / -0.02em tracking
Heading 1:  38px / 48px line-height / -0.01em tracking  
Heading 2:  30px / 40px line-height / -0.01em tracking
Heading 3:  24px / 32px line-height / 0em tracking
Body Large: 19px / 28px line-height / 0em tracking
Body:       16px / 24px line-height / 0em tracking
Small:      14px / 20px line-height / 0.01em tracking
Caption:    12px / 16px line-height / 0.02em tracking
```

### Primary Typeface: Inter

#### Weights & Usage
- **Light (300)**: Large display text only
- **Regular (400)**: Body text, UI elements
- **Medium (500)**: Emphasis, buttons, labels
- **SemiBold (600)**: Headings, important metrics
- **Bold (700)**: Avoid - too heavy for our aesthetic

#### Implementation
```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&display=swap');

body {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', 
               Roboto, Oxygen, Ubuntu, sans-serif;
  font-feature-settings: 'cv01', 'cv02', 'cv03', 'cv04', 'cv05', 
                        'cv06', 'cv07', 'cv08', 'cv09', 'cv10', 
                        'ss01', 'ss02', 'zero';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

### Display Typeface: Instrument Sans
Reserved for marketing materials and large headlines only.

### Monospace: JetBrains Mono
For all numerical data, code, and financial figures.

```css
.financial-figure {
  font-family: 'JetBrains Mono', 'Courier New', monospace;
  font-variant-numeric: tabular-nums;
}
```

---

## Color System

### Primary Palette

#### Trust Blue (Primary)
- **Hex**: #0A0F1C
- **RGB**: 10, 15, 28
- **Usage**: Primary text, logos, important UI elements

#### Flow Blue (Accent)
- **Hex**: #0066FF
- **RGB**: 0, 102, 255
- **Usage**: CTAs, links, active states, progress indicators

#### Pure White
- **Hex**: #FFFFFF
- **RGB**: 255, 255, 255
- **Usage**: Backgrounds, cards, negative space

#### Whisper Gray
- **Hex**: #F7F8FA
- **RGB**: 247, 248, 250
- **Usage**: Subtle backgrounds, disabled states

### Extended Palette

#### Grays
```
Gray 50:  #F9FAFB
Gray 100: #F3F4F6
Gray 200: #E5E7EB
Gray 300: #D1D5DB
Gray 400: #9CA3AF
Gray 500: #6B7280
Gray 600: #4B5563
Gray 700: #374151
Gray 800: #1F2937
Gray 900: #111827
```

### Functional Colors

#### Success (Teal)
- **Primary**: #00D4AA
- **Light**: #E6FFF9
- **Dark**: #008B6F

#### Warning (Amber)
- **Primary**: #FF6B00
- **Light**: #FFF4E6
- **Dark**: #CC5500

#### Error (Coral)
- **Primary**: #FF3366
- **Light**: #FFE6ED
- **Dark**: #CC1144

### Color Usage Principles
- **60-30-10 Rule**: 60% white/light gray, 30% Trust Blue, 10% accent colors
- **Monochromatic First**: Default to grayscale, add color purposefully
- **Functional Color**: Color should always convey meaning
- **Accessibility**: All color combinations must pass WCAG AA standards

---

## Visual Design System

### Grid System

#### Base Unit
8px - All spacing and sizing should be multiples of 8.

#### Spacing Scale
```
Space 0.5:  4px   (exceptions only)
Space 1:    8px   (tight spacing)
Space 2:    16px  (default spacing)
Space 3:    24px  (section spacing)
Space 4:    32px  (large spacing)
Space 5:    40px  (component spacing)
Space 6:    48px  (section breaks)
Space 8:    64px  (major sections)
Space 10:   80px  (hero spacing)
```

#### Layout Grid
- **Columns**: 12
- **Gutter**: 24px
- **Margin**: 24px (mobile), 40px (tablet), 80px (desktop)
- **Max Width**: 1120px

### Elevation System

We use elevation sparingly through subtle borders and shadows.

```css
/* Elevation 0 - Flat */
.elevation-0 {
  border: 1px solid #F0F0F0;
}

/* Elevation 1 - Raised */
.elevation-1 {
  border: 1px solid #E5E7EB;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
}

/* Elevation 2 - Floating */
.elevation-2 {
  border: 1px solid #E5E7EB;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.06);
}

/* Elevation 3 - Modal */
.elevation-3 {
  border: 1px solid #D1D5DB;
  box-shadow: 0 12px 24px rgba(0, 0, 0, 0.08);
}
```

### Border Radius
- **None**: 0px (preferred for ultimate minimalism)
- **Subtle**: 2px (for slight softness)
- **Round**: 9999px (for pills and badges only)

---

## Iconography

### Icon Specifications
- **Size**: 24x24px (base), 16x16px (small), 32x32px (large)
- **Stroke**: 2px (scales with size)
- **Style**: Rounded line caps and joins
- **Grid**: 24x24 with 2px padding

### Custom Icon Set
Essential icons designed specifically for FlowTrace:

```
Flow Node      Transaction    Verification    Trust Score
    ○             ━━━>           ✓               ◐
   ╱ ╲             $              📷              87
  ○   ○           ━━━>           GPS             ━━

Dashboard      Analytics      Settings        Export
  ┌─┬─┐          ╱╲            ⚙              ↗
  ├─┼─┤         ╱  ╲           ○              □→
  └─┴─┘        ╱____╲         ⚙⚙
```

### Icon Usage
- Always maintain 8px minimum touch target padding
- Use Trust Blue (#0A0F1C) as default color
- Flow Blue (#0066FF) for active/selected states
- Maintain consistent stroke width across all icons

---

## Motion Principles

### Timing
- **Micro**: 150ms (hovers, small state changes)
- **Macro**: 250ms (page transitions, major changes)
- **Slow**: 350ms (complex animations only)

### Easing Functions
```css
/* Default - Smooth Deceleration */
--ease-out: cubic-bezier(0.4, 0, 0.2, 1);

/* Entry - Gentle Acceleration */
--ease-in: cubic-bezier(0.4, 0, 1, 1);

/* Smooth - Balanced Motion */
--ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
```

### Animation Patterns

#### Hover States
```css
.button {
  transition: all 200ms var(--ease-out);
}

.button:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 102, 255, 0.15);
}
```

#### Loading States
- Subtle pulse for skeletons
- Linear progress bars (no bounce)
- Fade in content (opacity 0 to 1)

#### Page Transitions
- Fade + subtle slide (8px maximum)
- Stagger list items by 50ms
- No zooms or rotations

---

## Component Library

### Buttons

#### Primary Button
```css
.btn-primary {
  padding: 12px 24px;
  background: #0066FF;
  color: white;
  border: none;
  font-weight: 500;
  font-size: 16px;
  line-height: 24px;
  cursor: pointer;
  transition: all 200ms ease-out;
}

.btn-primary:hover {
  background: #0052CC;
  transform: translateY(-1px);
}
```

#### Secondary Button
```css
.btn-secondary {
  padding: 12px 24px;
  background: white;
  color: #0A0F1C;
  border: 1px solid #E5E7EB;
  font-weight: 500;
  transition: all 200ms ease-out;
}

.btn-secondary:hover {
  border-color: #0066FF;
  color: #0066FF;
}
```

### Cards

```css
.card {
  background: white;
  border: 1px solid #F0F0F0;
  padding: 32px;
  position: relative;
  overflow: hidden;
  transition: all 250ms ease-out;
}

.card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, #0066FF 0%, #00D4AA 100%);
  transform: translateX(-100%);
  transition: transform 250ms ease-out;
}

.card:hover::before {
  transform: translateX(0);
}
```

### Form Elements

#### Input Field
```css
.input {
  width: 100%;
  padding: 12px 16px;
  border: 1px solid #E5E7EB;
  background: white;
  font-size: 16px;
  transition: all 200ms ease-out;
}

.input:focus {
  outline: none;
  border-color: #0066FF;
  box-shadow: 0 0 0 3px rgba(0, 102, 255, 0.1);
}
```

### Data Visualization

#### Trust Score Display
```css
.trust-score {
  font-family: 'JetBrains Mono', monospace;
  font-size: 48px;
  font-weight: 300;
  letter-spacing: -0.02em;
}

.trust-score-bar {
  height: 4px;
  background: #F3F4F6;
  position: relative;
  overflow: hidden;
}

.trust-score-progress {
  height: 100%;
  background: linear-gradient(90deg, #0066FF 0%, #00D4AA 100%);
  transition: width 1000ms ease-out;
}
```

---

## Brand Applications

### Business Card Design
```
Front:
┌─────────────────────────────────┐
│                                 │
│                                 │
│     ○━━○━━○  FlowTrace         │
│                                 │
│                                 │
└─────────────────────────────────┘

Back:
┌─────────────────────────────────┐
│                                 │
│  Sarah Mwangi                   │
│  Trust Architect                │
│                                 │
│  sarah@flowtrace.io             │
│  +254 700 123 456               │
│                                 │
└─────────────────────────────────┘

Specifications:
- Size: 85mm x 55mm
- Paper: Uncoated 400gsm
- Finish: Matte
- Color: Trust Blue on Pure White
```

### Email Signature
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Sarah Mwangi
Trust Architect

○━━○━━○  FlowTrace
Building Financial Trust

+254 700 123 456
sarah@flowtrace.io
flowtrace.io
```

### Social Media Templates

#### LinkedIn Banner
- 1584 x 396px
- Logo left-aligned
- Tagline: "Financial Transparency for African Innovation"
- Subtle flow visualization in background

#### Twitter/X Profile
- Simplified logo mark only
- Trust Blue background
- No effects or gradients

### Presentation Template

#### Title Slide
```
┌─────────────────────────────────┐
│                                 │
│                                 │
│     Financial Transparency      │
│     for African Innovation      │
│                                 │
│     ○━━○━━○                    │
│                                 │
│                                 │
└─────────────────────────────────┘
```

#### Content Slide
```
┌─────────────────────────────────┐
│ Section Title                   │
│ ━━━━━━━━━━━━                   │
│                                 │
│ • Key point one                 │
│ • Key point two                 │
│ • Key point three               │
│                                 │
│        [Visualization]          │
└─────────────────────────────────┘
```

---

## Digital Guidelines

### Website Header
```css
.header {
  height: 64px;
  border-bottom: 1px solid #F0F0F0;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(8px);
  position: sticky;
  top: 0;
  z-index: 100;
}
```

### Mobile Responsive

#### Breakpoints
- **Mobile**: 320px - 767px
- **Tablet**: 768px - 1023px
- **Desktop**: 1024px+
- **Wide**: 1440px+

#### Touch Targets
- Minimum 44x44px
- 8px spacing between targets
- Clear active states

### Accessibility

#### Color Contrast
- Normal text: 4.5:1 minimum
- Large text: 3:1 minimum
- All combinations tested

#### Focus States
```css
:focus-visible {
  outline: 2px solid #0066FF;
  outline-offset: 2px;
}
```

---

## Do's and Don'ts

### Logo Do's ✓
- ✓ Use plenty of clear space
- ✓ Maintain minimum sizes
- ✓ Use approved color variations
- ✓ Keep proportions locked

### Logo Don'ts ✗
- ✗ Don't add effects or shadows
- ✗ Don't rotate or skew
- ✗ Don't change colors
- ✗ Don't recreate the logo

### Typography Do's ✓
- ✓ Use the type scale consistently
- ✓ Maintain hierarchy
- ✓ Use Inter for all UI text
- ✓ Apply proper line-height

### Typography Don't ✗
- ✗ Don't use decorative fonts
- ✗ Don't use more than 2 weights per page
- ✗ Don't center-align body text
- ✗ Don't use all caps for emphasis

### Color Do's ✓
- ✓ Use color purposefully
- ✓ Maintain 60-30-10 ratio
- ✓ Test accessibility
- ✓ Use functional colors consistently

### Color Don'ts ✗
- ✗ Don't use gradients in UI
- ✗ Don't create new colors
- ✗ Don't use color as the only indicator
- ✗ Don't use pure black (#000000)

### General Do's ✓
- ✓ Embrace white space
- ✓ Keep it simple
- ✓ Be consistent
- ✓ Design mobile-first

### General Don'ts ✗
- ✗ Don't add unnecessary elements
- ✗ Don't use stock photography
- ✗ Don't follow trends blindly
- ✗ Don't sacrifice clarity for style

---

## Conclusion

The FlowTrace brand is built on the principle that the most sophisticated solutions appear simple. Every element of our visual identity reinforces trust through clarity, precision, and restraint.

When in doubt, choose:
- Less over more
- Clear over clever
- Function over form
- Human over corporate

Our brand is not just how we look—it's how we make people feel. Confident. Trusted. Empowered.

---

### Version History
- v1.0 - June 2025 - Initial brand guidelines

### Contact
For brand questions: brand@flowtrace.io

© 2025 FlowTrace. All rights re