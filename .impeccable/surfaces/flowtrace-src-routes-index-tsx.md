---
version: 1
slug: "flowtrace-src-routes-index-tsx"
primary_target: "flowtrace/src/routes/index.tsx"
related_targets: ["flowtrace/src/components/layout/app-shell.tsx","flowtrace/src/styles.css"]
---

# Surface brief: landing page (and the app world it sets)

Scope: `/` landing page, Persuade mode. It establishes the visual world that the whole app (shell and every screen, Operate mode) inherits.
Audience: NGO executive directors and finance leads in Kenya and East Africa.
Action: Explore the live demo, which opens the app as Demo NGO Kenya with bundled demo data. Sign in is a quiet secondary link (auth is dummy for now).
Proof: the demo itself, using real seed figures labelled "Demo data". No invented customers, volumes, compliance claims or pricing.
Avoid: generic SaaS/fintech, African stereotypes, charity-ad emotion, cold corporate.

## Direction contract
THESIS: FlowTrace is the situation report for an org's own money, a live sitrep that NGO leaders already know how to read. It refuses the SaaS split hero (headline beside a dashboard screenshot over a feature bento).
OWN-WORLD: Cool off-white report paper (#F5F6F3), ink navy (#13202D), one report blue (#1C5FA8), gap grey for unfunded coverage, signal orange only for flags. Heavy 3px ink rules open every section; hairlines divide figures. Archivo, a variable grotesque with a width axis: condensed and heavy for figures and headlines, normal width for prose. Tabular numerals everywhere. Square corners. Verification state is drawn by line pattern: solid = evidenced, dashed = awaiting proof, broken with a cross = flagged.
STORY: The visitor sees their own reporting ritual applied to their money. They understand that each movement carries proof and that funders get a live view, then open the demo.
FIRST VIEWPORT: Masthead strip (wordmark, "Situation report · Demo NGO Kenya · as of <date>", nav, Explore the demo). Left 5/12: kicker, condensed headline at about 76px, lede, primary "Explore the demo" plus a quiet "How proof is attached". Right 7/12: the report: key figures in 4 cells, funding received by donor as coverage bars (pending shown dashed), and the line-pattern legend. A 3px rule and "01 Money moves / 02 Proof attaches / 03 Funders see it live" start at the fold.
FORM: Situation Report (OCHA sitrep / FTS), candidate 6 of 7 on my grounded list. Raises: stroke-pattern state (jackfield), a strict module grid with history ghosted (brick instructions), select-to-isolate with weight = amount (event display), type size carries magnitude (Massin). seed: none (unavailable)
SIGNATURE INTERACTION: hovering or focusing a donor row isolates its flow and dims the rest. In the app, selecting any flow edge isolates it the same way.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
