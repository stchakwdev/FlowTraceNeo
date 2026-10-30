#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use serde_json::Value;

use super::*;
use crate::backend::demo_transparency;
use crate::bills::days_between;
use crate::dashboard::transactions;
use ledger_core::types::{EntryId, FlowNodeId};

/// The web's own output over the demo transactions (`rust/tools/src/flow-graph.ts`).
const FIXTURE: &str = include_str!("../../../../fixtures/transparency-flow-graph.json");

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}

fn period_of(id: &str) -> PeriodId {
    match id {
        "week" => PeriodId::Week,
        "month" => PeriodId::Month,
        "quarter" => PeriodId::Quarter,
        _ => PeriodId::Year,
    }
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

/// A web amount in major units as minor units.
fn minor(v: &Value) -> Minor {
    v.as_i64().unwrap() * 100
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect()
}

fn status_of(s: &str) -> TransactionStatus {
    serde_json::from_value(Value::String(s.to_owned())).unwrap()
}

/// `M x1 y1 C mx y1, mx y2, x2 y2` back into its numbers.
fn path_of(s: &str) -> EdgePath {
    let n: Vec<f64> = s.split([' ', ',']).filter_map(|p| p.parse().ok()).collect();
    assert_eq!(n.len(), 8, "{s}");
    assert_eq!((n[1], n[2], n[5]), (n[3], n[4], n[7]), "{s}");
    EdgePath {
        x1: n[0],
        y1: n[1],
        mx: n[2],
        x2: n[6],
        y2: n[7],
    }
}

#[test]
fn constants_match_the_web() {
    let fx = fixture();
    let c = &fx["constants"];
    assert_eq!(f(&c["node_w"]), NODE_W);
    for (p, w) in PERIODS.iter().zip(c["periods"].as_array().unwrap()) {
        assert_eq!(p.label, w["label"].as_str().unwrap());
        assert_eq!(p.days, w["days"].as_i64().unwrap());
        assert_eq!(p.phrase, w["phrase"].as_str().unwrap());
    }
    for ((label, types), w) in BANDS.iter().zip(c["bands"].as_array().unwrap()) {
        assert_eq!(*label, w["label"].as_str().unwrap());
        let names: Vec<String> = types
            .iter()
            .map(|t| format!("{t:?}").to_lowercase())
            .collect();
        assert_eq!(names, strings(&w["types"]));
    }
    for t in [
        MapNodeType::Donor,
        MapNodeType::Account,
        MapNodeType::Department,
        MapNodeType::Project,
        MapNodeType::Vendor,
        MapNodeType::Beneficiary,
        MapNodeType::Unassigned,
    ] {
        let key = format!("{t:?}").to_lowercase();
        assert_eq!(t.label(), c["node_type_label"][key].as_str().unwrap());
    }
}

#[test]
fn wrap_name_matches_every_web_case() {
    for case in fixture()["wrap_name"].as_array().unwrap() {
        let name = case[0].as_str().unwrap();
        assert_eq!(wrap_name(name), strings(&case[1]), "{name:?}");
    }
}

#[test]
fn every_period_builds_the_webs_map_exactly() {
    let t = demo_transparency().unwrap();
    let all = transactions(&t);
    for p in fixture()["periods"].as_array().unwrap() {
        let period = period_of(p["period"].as_str().unwrap());
        let window = period_window(&all, period).unwrap();
        assert_eq!(
            format!("{}T00:00:00.000Z", window.start.as_str()),
            p["window"]["start"].as_str().unwrap()
        );
        assert_eq!(
            format!("{}T00:00:00.000Z", window.end.as_str()),
            p["window"]["end"].as_str().unwrap()
        );
        assert_eq!(
            days_between(&window.start, &window.end) + 1,
            period.period().days
        );
        let inside = filter_by_period(&all, period);
        let ids: Vec<String> = inside.iter().map(|x| x.transaction.id.clone()).collect();
        assert_eq!(ids, strings(&p["transaction_ids"]), "{period:?}");

        let graph = build_flow_graph(&inside);
        let web = &p["graph"];
        assert_graph(&graph, web, period);

        for c in p["connected"].as_array().unwrap() {
            let id = c["selection"]["id"].as_str().unwrap().to_owned();
            let sel = if c["selection"]["kind"] == "node" {
                Selection::Node(id)
            } else {
                Selection::Edge(id)
            };
            let set = connected_set(&graph, Some(&sel)).unwrap();
            assert_eq!(set.nodes, strings(&c["nodes"]), "{sel:?}");
            assert_eq!(set.edges, strings(&c["edges"]), "{sel:?}");
        }
    }
}

fn assert_graph(graph: &FlowGraph, web: &Value, period: PeriodId) {
    assert_eq!(graph.width, f(&web["width"]), "{period:?}");
    assert_eq!(graph.height, f(&web["height"]), "{period:?}");
    let totals = &web["totals"];
    assert_eq!(graph.totals.moved, minor(&totals["moved"]));
    assert_eq!(graph.totals.evidenced, minor(&totals["evidenced"]));
    assert_eq!(
        graph.totals.movements,
        usize::try_from(totals["movements"].as_u64().unwrap()).unwrap()
    );

    let columns: Vec<(f64, Option<&str>)> = web["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (f(&c["x"]), c["label"].as_str()))
        .collect();
    let ours: Vec<(f64, Option<&str>)> = graph.columns.iter().map(|c| (c.x, c.label)).collect();
    assert_eq!(ours, columns, "{period:?}");

    let nodes = web["nodes"].as_array().unwrap();
    assert_eq!(graph.nodes.len(), nodes.len(), "{period:?}");
    for (n, w) in graph.nodes.iter().zip(nodes) {
        assert_eq!(n.id, w["id"].as_str().unwrap());
        assert_eq!(n.name, w["name"].as_str().unwrap());
        assert_eq!(
            format!("{:?}", n.node_type).to_lowercase(),
            w["type"].as_str().unwrap()
        );
        assert_eq!(
            n.band,
            usize::try_from(w["band"].as_u64().unwrap()).unwrap()
        );
        assert_eq!(
            n.column,
            usize::try_from(w["column"].as_u64().unwrap()).unwrap(),
            "{} {period:?}",
            n.name
        );
        assert_eq!(
            (n.inflow, n.outflow),
            (minor(&w["inflow"]), minor(&w["outflow"]))
        );
        assert_eq!(
            (n.x, n.y, n.h),
            (f(&w["x"]), f(&w["y"]), f(&w["h"])),
            "{}",
            n.name
        );
        assert_eq!(n.lines, strings(&w["lines"]));
    }

    let edges = web["edges"].as_array().unwrap();
    assert_eq!(graph.edges.len(), edges.len(), "{period:?}");
    for (e, w) in graph.edges.iter().zip(edges) {
        assert_eq!(e.id, w["id"].as_str().unwrap());
        assert_eq!(
            (e.source.as_str(), e.target.as_str()),
            (w["source"].as_str().unwrap(), w["target"].as_str().unwrap())
        );
        assert_eq!(e.amount, minor(&w["amount"]));
        assert_eq!(e.status, status_of(w["status"].as_str().unwrap()));
        assert_eq!(e.width, f(&w["width"]), "{}", e.id);
        assert_eq!(e.path, path_of(w["path"].as_str().unwrap()), "{}", e.id);
        assert_eq!(e.mid, (f(&w["mid"]["x"]), f(&w["mid"]["y"])));
        let ids: Vec<String> = e
            .transactions
            .iter()
            .map(|t| t.transaction.id.clone())
            .collect();
        assert_eq!(ids, strings(&w["transaction_ids"]));
    }
}

fn demo_page(period: PeriodId) -> FlowPage {
    FlowPage::of(&demo_transparency().unwrap(), period)
}

#[test]
fn the_page_texts_follow_the_web() {
    let page = demo_page(PeriodId::Month);
    assert_eq!(
        page.lede(),
        "Movements from 6 Jan 2026 to 4 Feb 2026, counted back from the latest recorded \
         transaction."
    );
    assert_eq!(page.body(), Body::Map);
    let figures: Vec<(String, String, Option<String>)> = page
        .figures()
        .into_iter()
        .map(|f| (f.value, f.label, f.detail))
        .collect();
    assert_eq!(
        figures,
        [
            (
                "KES 23.9M".to_owned(),
                "Total moved".to_owned(),
                Some("KES 23,932,500".to_owned())
            ),
            ("18".to_owned(), "Nodes".to_owned(), None),
            (
                "15".to_owned(),
                "Links".to_owned(),
                Some("15 movements".to_owned())
            ),
            (
                "68%".to_owned(),
                "Share evidenced".to_owned(),
                Some("KES 16,217,500 has proof".to_owned())
            ),
        ]
    );

    let empty = FlowPage::of(&OrgTransparency::default(), PeriodId::Month);
    assert_eq!(empty.body(), Body::Empty);
    assert_eq!(empty.lede(), LEDE_NONE);
    assert!(!empty.has_movements());
}

#[test]
fn a_period_with_nothing_in_it_offers_the_widest() {
    let mut t = demo_transparency().unwrap();
    // With no source recorded nothing is drawn (`if (!t.from_node) continue`), while the
    // movements still set the window.
    for x in &mut t.transactions {
        x.from_node_id = None;
    }
    let page = FlowPage::of(&t, PeriodId::Week);
    assert_eq!(
        page.body(),
        Body::EmptyPeriod {
            title: "No movements in the last week".to_owned(),
            body: Some(
                "The week view covers 29 Jan 2026 to 4 Feb 2026, and nothing was recorded in it."
                    .to_owned()
            ),
            widen: Some("Show the last year".to_owned()),
        }
    );
    let year = FlowPage::of(&t, PeriodId::Year);
    assert!(matches!(year.body(), Body::EmptyPeriod { widen: None, .. }));
}

#[test]
fn zoom_steps_and_fits_as_the_web() {
    // A 1172-wide map in a 900-wide container: legible fit stops at 0.9, Fit goes lower.
    assert_eq!(Zoom::Legible.scale(900.0, 1172.0), LEGIBLE_MIN);
    let fit = Zoom::Fit.scale(900.0, 1172.0);
    assert!((fit - 898.0 / 1172.0).abs() < 1e-12);
    assert_eq!(Zoom::Legible.scale(2000.0, 1172.0), 1.0, "never above 1");
    assert_eq!(Zoom::Fit.scale(100.0, 1172.0), ZOOM_MIN);
    assert_eq!(Zoom::Legible.scale(0.0, 1172.0), 1.0, "unmeasured");
    assert_eq!(Zoom::step(0.9, true), Zoom::Level(1.0));
    assert_eq!(Zoom::step(fit, false), Zoom::Level(0.7));
    assert_eq!(Zoom::step(1.6, true), Zoom::Level(ZOOM_MAX));
    assert_eq!(Zoom::step(0.4, false), Zoom::Level(ZOOM_MIN));
    assert_eq!(Zoom::percent(0.9), "90%");
    assert_eq!(Zoom::percent(fit), "77%");
}

#[test]
fn edge_details_list_movements_newest_first_with_ledger_links() {
    let page = demo_page(PeriodId::Year);
    let g = &page.graph;
    let single = g.edges.iter().find(|e| e.transactions.len() == 1).unwrap();
    let d = Details::of(g, &Selection::Edge(single.id.clone())).unwrap();
    assert!(d.heading.contains(" \u{2192} "));
    assert_eq!(d.subtitle, "1 movement on this line");
    assert_eq!(d.figures[0].0, "Moved in period");
    assert_eq!(
        d.ledger,
        Some((
            "Ledger lines for this movement".to_owned(),
            LedgerLink::Source(single.transactions[0].transaction.id.clone())
        ))
    );
    assert_eq!(d.lists[0].title, "Movements");
    assert!(d.lists[0].items.iter().all(|i| i.ledger.is_none()));
    assert!(d.lists[0].items.iter().all(|i| i.direction.is_none()));

    let unassigned = Details::of(g, &Selection::Node(UNASSIGNED_ID.to_owned())).unwrap();
    assert_eq!(unassigned.heading, UNASSIGNED_NAME);
    assert_eq!(
        unassigned.subtitle,
        "No recipient. These movements left an account without a recipient on record."
    );
    assert_eq!(unassigned.ledger, None);
    assert_eq!(unassigned.lists[0].title, "Money in (1)");

    let fund = g.nodes.iter().find(|n| n.name == "General Fund").unwrap();
    let d = Details::of(g, &Selection::Node(fund.id.clone())).unwrap();
    assert_eq!(d.subtitle, "Account");
    assert_eq!(
        d.ledger,
        Some((
            "Ledger lines for General Fund".to_owned(),
            LedgerLink::Node(fund.id.clone())
        ))
    );
    let out = d
        .lists
        .iter()
        .find(|l| l.title.starts_with("Money out"))
        .unwrap();
    assert_eq!(out.title, "Money out (3)");
    let dates: Vec<&str> = out.items.iter().map(|i| i.date.as_str()).collect();
    assert_eq!(dates, ["31 Jan 2026", "12 Jan 2026", "12 Jan 2026"]);
    assert!(out.items.iter().all(|i| i.direction == Some("to")));
    assert!(Details::of(g, &Selection::Node("nope".to_owned())).is_none());
}

#[test]
fn hits_and_tab_order_follow_the_drawing() {
    let page = demo_page(PeriodId::Year);
    let g = &page.graph;
    let n = &g.nodes[0];
    assert_eq!(
        g.hit(n.x + 10.0, n.y + 10.0),
        Some(Selection::Node(n.id.clone()))
    );
    let e = &g.edges[0];
    let (mx, my) = e.mid;
    assert_eq!(g.hit(mx, my), Some(Selection::Edge(e.id.clone())));
    assert_eq!(g.hit(1.0, 1.0), None, "the background");

    let order = g.focus_order();
    assert_eq!(order.len(), g.nodes.len() + g.edges.len());
    assert!(matches!(&order[0], Selection::Node(id) if g.node(id).unwrap().column == 0));
    let last_node = order
        .iter()
        .rposition(|s| matches!(s, Selection::Node(_)))
        .unwrap();
    let first_recipient = g
        .nodes
        .iter()
        .find(|n| n.column == g.columns.len() - 1)
        .unwrap();
    assert!(order[..=last_node].contains(&Selection::Node(first_recipient.id.clone())));
}

#[test]
fn dashes_scale_with_the_line_and_the_cross_with_its_width() {
    assert_eq!(edge_dash(TransactionStatus::Verified, 10.0), None);
    assert_eq!(edge_dash(TransactionStatus::Pending, 2.0), Some([6.0, 5.0]));
    assert_eq!(
        edge_dash(TransactionStatus::Flagged, 6.0),
        Some([28.0, 16.0])
    );
    assert_eq!(
        edge_dash(TransactionStatus::Rejected, 9.0),
        Some([6.0, 18.0])
    );
    assert_eq!(cross_size(2.0), 5.0);
    assert_eq!(cross_size(10.0), 8.0);
    let node = GraphNode {
        id: "n".to_owned(),
        name: "n".to_owned(),
        node_type: MapNodeType::Account,
        band: 1,
        column: 0,
        inflow: 85_000_000,
        outflow: 2_000_000,
        x: 0.0,
        y: 0.0,
        lines: vec![],
        h: 60.0,
    };
    assert_eq!(node_amount(&node), ("in KES ", "850K".to_owned()));
    assert_eq!(meta_baseline(&node), 47.0);
    assert_eq!(name_baseline(2), 61.0);
}

#[test]
fn a_ledger_link_opens_the_linked_entries_else_the_source() {
    let mut t = demo_transparency().unwrap();
    let id = t.transactions[0].id.clone();
    let source = LedgerLink::Source(id.clone()).filter(Some(&t));
    assert_eq!(
        source.source,
        Some(crate::general_ledger::GlSource {
            id: id.clone(),
            entries: vec![]
        }),
        "the demo's movements carry no links, so the source id resolves"
    );
    assert_eq!(source.node, None);
    t.transactions[0].journal_entry_ids = vec!["e1".to_owned(), "e2".to_owned()];
    let linked = LedgerLink::Source(id).filter(Some(&t));
    let entries: Vec<&str> = linked
        .source
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .map(EntryId::as_str)
        .collect();
    assert_eq!(entries, ["e1", "e2"]);
    let node = LedgerLink::Node("n1".to_owned()).filter(None);
    assert_eq!(node.node.as_ref().map(FlowNodeId::as_str), Some("n1"));
    assert_eq!(node.source, None);
}
