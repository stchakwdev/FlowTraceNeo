//! The Flows screen's view models (DT2), plain Rust with no GPUI: the web's flow map
//! (`flowtrace/src/components/flows/flow-graph.ts`, ported function by function) and the page
//! around it (`flowtrace/src/routes/_authenticated/flows.tsx`, `components/flows/flow-map.tsx`
//! and `flow-details.tsx`) over the organisation's movements between flow nodes. Amounts are
//! minor units where the web holds major units; the layout is in the web's map units, so the
//! fixture `rust/fixtures/transparency-flow-graph.json` pins every coordinate.

use std::cmp::Ordering;
use std::collections::HashMap;

use ledger_core::Minor;
use ledger_core::types::{EntryId, FlowNodeId, IsoDate};
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::flows::{
    NodeRef, NodeType, TransactionStatus, TransactionWithNodes,
};

use crate::bills::add_days;
use crate::dashboard::{Figure, proof_label};
use crate::general_ledger::{GlFilter, GlSource};
use crate::web_format::{format_compact, format_date, format_money};

// ---------------------------------------------------------------------------------------------
// Periods
// ---------------------------------------------------------------------------------------------

/// A period of the map (`PeriodId`, `flow-graph.ts:8-15`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeriodId {
    /// Seven days.
    Week,
    /// Thirty days.
    Month,
    /// Ninety-one days.
    Quarter,
    /// 365 days.
    Year,
}

/// One entry of `PERIODS` (`flow-graph.ts:8-13`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    /// Which.
    pub id: PeriodId,
    /// The control's label.
    pub label: &'static str,
    /// Days counted back, the latest day included.
    pub days: i64,
    /// `last month`, for the empty notice.
    pub phrase: &'static str,
}

/// `PERIODS` (`flow-graph.ts:8-13`), in the control's order.
pub const PERIODS: [Period; 4] = [
    Period {
        id: PeriodId::Week,
        label: "Week",
        days: 7,
        phrase: "last week",
    },
    Period {
        id: PeriodId::Month,
        label: "Month",
        days: 30,
        phrase: "last month",
    },
    Period {
        id: PeriodId::Quarter,
        label: "Quarter",
        days: 91,
        phrase: "last quarter",
    },
    Period {
        id: PeriodId::Year,
        label: "Year",
        days: 365,
        phrase: "last year",
    },
];

/// `WIDEST_PERIOD` (`flow-graph.ts:16`).
pub const WIDEST_PERIOD: PeriodId = PeriodId::Year;

impl PeriodId {
    /// Its entry of [`PERIODS`].
    #[must_use]
    pub fn period(self) -> Period {
        match self {
            Self::Week => PERIODS[0],
            Self::Month => PERIODS[1],
            Self::Quarter => PERIODS[2],
            Self::Year => PERIODS[3],
        }
    }
}

/// The days a period covers, both included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    /// First day.
    pub start: IsoDate,
    /// Last day: the latest movement's.
    pub end: IsoDate,
}

/// `dayValue` (`flow-graph.ts:21-23`): the calendar day of a date or timestamp; none for a value
/// that is not one (the web's `NaN`, which no comparison lets through).
fn day_value(date: &str) -> Option<IsoDate> {
    IsoDate::parse(date.get(..10)?).ok()
}

/// `periodWindow` (`flow-graph.ts:29-38`): counted back from the latest transaction date in the
/// data, not from today, so a quiet dataset still shows its recent work.
#[must_use]
pub fn period_window(transactions: &[TransactionWithNodes], period: PeriodId) -> Option<Window> {
    let end = transactions
        .iter()
        .filter_map(|t| day_value(&t.transaction.transaction_date))
        .max()?;
    let start = add_days(&end, -(period.period().days - 1));
    Some(Window { start, end })
}

/// `filterByPeriod` (`flow-graph.ts:40-52`): the movements inside the period's window, in the
/// order given.
#[must_use]
pub fn filter_by_period(
    transactions: &[TransactionWithNodes],
    period: PeriodId,
) -> Vec<TransactionWithNodes> {
    let Some(w) = period_window(transactions, period) else {
        return Vec::new();
    };
    transactions
        .iter()
        .filter(|t| {
            day_value(&t.transaction.transaction_date).is_some_and(|d| d >= w.start && d <= w.end)
        })
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------------------------

/// A node's kind on the map (`NodeType`, `flow-graph.ts:58-65`): the flow node types and the
/// stand-in for a movement with no recipient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapNodeType {
    /// `donor`.
    Donor,
    /// `account`.
    Account,
    /// `department`.
    Department,
    /// `project`.
    Project,
    /// `vendor`.
    Vendor,
    /// `beneficiary`.
    Beneficiary,
    /// No recipient recorded.
    Unassigned,
}

impl From<NodeType> for MapNodeType {
    fn from(t: NodeType) -> Self {
        match t {
            NodeType::Donor => Self::Donor,
            NodeType::Account => Self::Account,
            NodeType::Department => Self::Department,
            NodeType::Project => Self::Project,
            NodeType::Vendor => Self::Vendor,
            NodeType::Beneficiary => Self::Beneficiary,
        }
    }
}

impl MapNodeType {
    /// `NODE_TYPE_LABEL` (`flow-graph.ts:75-83`).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Donor => "Donor",
            Self::Account => "Account",
            Self::Department => "Department",
            Self::Project => "Project",
            Self::Vendor => "Vendor",
            Self::Beneficiary => "Beneficiary",
            Self::Unassigned => "No recipient",
        }
    }

    /// `bandOf` (`flow-graph.ts:85-88`): its index in [`BANDS`].
    #[must_use]
    pub fn band(self) -> usize {
        BANDS
            .iter()
            .position(|(_, types)| types.contains(&self))
            .unwrap_or(1)
    }
}

/// `BANDS` (`flow-graph.ts:68-73`), money order: donors, accounts, departments and projects,
/// then recipients.
pub const BANDS: [(&str, &[MapNodeType]); 4] = [
    ("Donors", &[MapNodeType::Donor]),
    ("Accounts", &[MapNodeType::Account]),
    (
        "Departments, projects",
        &[MapNodeType::Department, MapNodeType::Project],
    ),
    (
        "Recipients",
        &[
            MapNodeType::Vendor,
            MapNodeType::Beneficiary,
            MapNodeType::Unassigned,
        ],
    ),
];

/// `UNASSIGNED_ID` (`flow-graph.ts:90`).
pub const UNASSIGNED_ID: &str = "__no-recipient__";

/// The unassigned node's name (`flow-graph.ts:206`).
pub const UNASSIGNED_NAME: &str = "No recipient recorded";

/// `NODE_W` (`flow-graph.ts:132`).
pub const NODE_W: f64 = 196.0;
/// `NODE_PAD`.
pub const NODE_PAD: f64 = 12.0;
/// `NAME_SIZE`: name type size in map units.
pub const NAME_SIZE: f64 = 14.0;
/// `NAME_LEADING`.
pub const NAME_LEADING: f64 = 18.0;
const NAME_TOP: f64 = 25.0;
const META_GAP: f64 = 22.0;
const META_BOTTOM: f64 = 13.0;
const MAX_LINES: usize = 3;
const CHAR_W: f64 = NAME_SIZE * 0.56;
const COL_GAP: f64 = 40.0;
const ROW_GAP: f64 = 18.0;
const PAD_X: f64 = 16.0;
const PAD_BOTTOM: f64 = 16.0;
const TOP: f64 = 40.0;
const MIN_STROKE: f64 = 2.0;
const MAX_STROKE: f64 = 16.0;

/// A node of the map (`GraphNode`, `flow-graph.ts:92-105`), amounts in minor units.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    /// Flow node id, or [`UNASSIGNED_ID`].
    pub id: String,
    /// Name.
    pub name: String,
    /// Kind.
    pub node_type: MapNodeType,
    /// Its band.
    pub band: usize,
    /// Its column.
    pub column: usize,
    /// Money in during the period.
    pub inflow: Minor,
    /// Money out.
    pub outflow: Minor,
    /// Left edge, map units.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// The name wrapped to the node's width, one entry per line.
    pub lines: Vec<String>,
    /// Height.
    pub h: f64,
}

/// The cubic of an edge, `M x1 y1 C mx y1, mx y2, x2 y2` (`flow-graph.ts:404`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgePath {
    /// Start x: the source's right edge.
    pub x1: f64,
    /// Start y: the source's port.
    pub y1: f64,
    /// The x of both control points.
    pub mx: f64,
    /// End x: the target's left edge.
    pub x2: f64,
    /// End y: the target's port.
    pub y2: f64,
}

impl EdgePath {
    /// The point at `t` (0 to 1) along the curve.
    #[must_use]
    pub fn at(&self, t: f64) -> (f64, f64) {
        let u = 1.0 - t;
        let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        let x = b0 * self.x1 + b1 * self.mx + b2 * self.mx + b3 * self.x2;
        let y = b0 * self.y1 + b1 * self.y1 + b2 * self.y2 + b3 * self.y2;
        (x, y)
    }

    /// The shortest distance from `(px, py)` to the curve, by 64 samples refined around the
    /// best.
    #[must_use]
    pub fn distance(&self, px: f64, py: f64) -> f64 {
        let gap = |at: f64| {
            let (cx, cy) = self.at(at);
            (cx - px).hypot(cy - py)
        };
        let steps = 64_u32;
        let (mut best_at, mut best) = (0.0, f64::INFINITY);
        for i in 0..=steps {
            let at = f64::from(i) / f64::from(steps);
            let here = gap(at);
            if here < best {
                (best_at, best) = (at, here);
            }
        }
        let mut span = 1.0 / f64::from(steps);
        for _ in 0..12 {
            for at in [best_at - span / 2.0, best_at + span / 2.0] {
                let at = at.clamp(0.0, 1.0);
                let here = gap(at);
                if here < best {
                    (best_at, best) = (at, here);
                }
            }
            span /= 2.0;
        }
        best
    }
}

/// A line of the map: every movement from one node to another (`GraphEdge`,
/// `flow-graph.ts:107-117`).
#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    /// `source->target`.
    pub id: String,
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Sum moved, minor units.
    pub amount: Minor,
    /// The worst proof state among its movements.
    pub status: TransactionStatus,
    /// Its movements, in the order given.
    pub transactions: Vec<TransactionWithNodes>,
    /// Stroke width, map units.
    pub width: f64,
    /// The curve.
    pub path: EdgePath,
    /// The curve's middle, for the flagged cross and the amount tag.
    pub mid: (f64, f64),
}

/// A column of the map (`FlowGraph.columns`): its left edge and the band label over the first
/// column of each band.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    /// Left edge.
    pub x: f64,
    /// Band label, on a band's first column.
    pub label: Option<&'static str>,
}

/// `FlowGraph.totals`, minor units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Totals {
    /// Everything drawn.
    pub moved: Minor,
    /// The verified part.
    pub evidenced: Minor,
    /// Movements in the period, drawn or not.
    pub movements: usize,
}

/// `FlowGraph` (`flow-graph.ts:119-130`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlowGraph {
    /// Nodes, in the order movements first touched them.
    pub nodes: Vec<GraphNode>,
    /// Edges, in the order their first movement came.
    pub edges: Vec<GraphEdge>,
    /// Columns, left to right.
    pub columns: Vec<Column>,
    /// Map width, map units.
    pub width: f64,
    /// Map height.
    pub height: f64,
    /// Totals.
    pub totals: Totals,
}

impl FlowGraph {
    /// The node of `id`.
    #[must_use]
    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// The edge of `id`.
    #[must_use]
    pub fn edge(&self, id: &str) -> Option<&GraphEdge> {
        self.edges.iter().find(|e| e.id == id)
    }

    /// Whether `selection` names something on this map (`flows.tsx:59-65`).
    #[must_use]
    pub fn contains(&self, selection: &Selection) -> bool {
        match selection {
            Selection::Node(id) => self.node(id).is_some(),
            Selection::Edge(id) => self.edge(id).is_some(),
        }
    }

    /// Tab order, "in money order" (`flow-map.tsx:89-114`): column by column, a column's nodes
    /// then the lines leaving it.
    #[must_use]
    pub fn focus_order(&self) -> Vec<Selection> {
        let mut out = Vec::new();
        for ci in 0..self.columns.len() {
            for n in self.nodes.iter().filter(|n| n.column == ci) {
                out.push(Selection::Node(n.id.clone()));
            }
            for e in &self.edges {
                if self.node(&e.source).is_some_and(|n| n.column == ci) {
                    out.push(Selection::Edge(e.id.clone()));
                }
            }
        }
        out
    }

    /// What a click at `(x, y)` in map units selects: a node whose box holds the point, else
    /// the nearest edge within its hit area (`max(16, width + 8)` wide, `flow-map.tsx:262-268`),
    /// else nothing (the background clears, `flow-map.tsx:71-76`).
    #[must_use]
    pub fn hit(&self, x: f64, y: f64) -> Option<Selection> {
        if let Some(n) = self
            .nodes
            .iter()
            .find(|n| x >= n.x && x <= n.x + NODE_W && y >= n.y && y <= n.y + n.h)
        {
            return Some(Selection::Node(n.id.clone()));
        }
        self.edges
            .iter()
            .map(|e| (e, e.path.distance(x, y)))
            .filter(|(e, d)| *d <= f64::max(16.0, e.width + 8.0) / 2.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(e, _)| Selection::Edge(e.id.clone()))
    }
}

/// `wrapName` (`flow-graph.ts:153-178`): at most three lines that fit inside a node; a word
/// longer than a line is cut hard with a hyphen, a fourth line becomes an ellipsis.
#[must_use]
pub fn wrap_name(name: &str) -> Vec<String> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Math.floor of a small positive width"
    )]
    let per_line = ((NODE_W - NODE_PAD * 2.0) / CHAR_W).floor() as usize;
    let len = |s: &str| s.encode_utf16().count();
    let cut = |s: &str, n: usize| -> String { s.chars().take(n).collect() };
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in name.split_whitespace() {
        let next = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if len(&next) <= per_line {
            line = next;
        } else {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            line = if len(word) > per_line {
                format!("{}-", cut(word, per_line - 1))
            } else {
                word.to_owned()
            };
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.len() > MAX_LINES {
        lines.truncate(MAX_LINES);
        let last = &lines[MAX_LINES - 1];
        let kept = if len(last) >= per_line {
            format!("{}\u{2026}", cut(last, per_line - 1).trim_end())
        } else {
            format!("{last}\u{2026}")
        };
        lines[MAX_LINES - 1] = kept;
        return lines;
    }
    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

/// `nodeHeight` (`flow-graph.ts:181-183`).
#[expect(clippy::cast_precision_loss, reason = "at most three lines")]
fn node_height(lines: usize) -> f64 {
    NAME_TOP + (lines as f64 - 1.0) * NAME_LEADING + META_GAP + META_BOTTOM
}

/// `nameBaseline` (`flow-graph.ts:186-188`): a name line's baseline below the node's top.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "at most three lines")]
pub fn name_baseline(line: usize) -> f64 {
    NAME_TOP + line as f64 * NAME_LEADING
}

/// `metaBaseline` (`flow-graph.ts:189-191`): the type and amount baseline below the top.
#[must_use]
pub fn meta_baseline(node: &GraphNode) -> f64 {
    node.h - META_BOTTOM
}

/// `STATUS_RANK` (`flow-graph.ts:193-198`).
fn rank(status: TransactionStatus) -> u8 {
    match status {
        TransactionStatus::Verified => 0,
        TransactionStatus::Pending => 1,
        TransactionStatus::Flagged => 2,
        TransactionStatus::Rejected => 3,
    }
}

/// `touch` (`flow-graph.ts:204-243`): the node a movement end lands on. A reference makes or
/// finds its node; no reference and no id is the unassigned node; an id without a reference (a
/// node out of reach) is none.
fn touch(
    nodes: &mut Vec<GraphNode>,
    index: &mut HashMap<String, usize>,
    r: Option<&NodeRef>,
    id: Option<&str>,
) -> Option<usize> {
    let (key, name, node_type) = match r {
        Some(r) => (r.id.clone(), r.name.clone(), MapNodeType::from(r.node_type)),
        None if id.is_some() => return None,
        None => (
            UNASSIGNED_ID.to_owned(),
            UNASSIGNED_NAME.to_owned(),
            MapNodeType::Unassigned,
        ),
    };
    if let Some(&at) = index.get(&key) {
        return Some(at);
    }
    index.insert(key.clone(), nodes.len());
    nodes.push(GraphNode {
        id: key,
        name,
        node_type,
        band: node_type.band(),
        column: 0,
        inflow: 0,
        outflow: 0,
        x: 0.0,
        y: 0.0,
        lines: Vec::new(),
        h: 0.0,
    });
    Some(nodes.len() - 1)
}

/// An edge before layout: source and target as node indexes.
struct Draft {
    source: usize,
    target: usize,
    amount: Minor,
    status: TransactionStatus,
    transactions: Vec<TransactionWithNodes>,
}

/// `buildFlowGraph` (`flow-graph.ts:200-418`): one edge per source and target, nodes in
/// columns by band (a node fed from its own band one sub-column right), rows ordered to keep
/// lines from crossing, line weight on a square-root scale of the amount, ports spread evenly.
#[must_use]
pub fn build_flow_graph(transactions: &[TransactionWithNodes]) -> FlowGraph {
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut drafts: Vec<Draft> = Vec::new();
    let mut edge_at: HashMap<(usize, usize), usize> = HashMap::new();
    let mut totals = Totals {
        movements: transactions.len(),
        ..Totals::default()
    };
    for t in transactions {
        let x = &t.transaction;
        let Some(from) = t.from_node.as_ref() else {
            continue;
        };
        let source = touch(
            &mut nodes,
            &mut index,
            Some(from),
            x.from_node_id.as_deref(),
        );
        let target = touch(
            &mut nodes,
            &mut index,
            t.to_node.as_ref(),
            x.to_node_id.as_deref(),
        );
        let (Some(source), Some(target)) = (source, target) else {
            continue;
        };
        let amount = x.amount;
        totals.moved += amount;
        if x.verification_status == TransactionStatus::Verified {
            totals.evidenced += amount;
        }
        nodes[source].outflow += amount;
        nodes[target].inflow += amount;
        if let Some(&at) = edge_at.get(&(source, target)) {
            let d = &mut drafts[at];
            d.amount += amount;
            d.transactions.push(t.clone());
            if rank(x.verification_status) > rank(d.status) {
                d.status = x.verification_status;
            }
        } else {
            edge_at.insert((source, target), drafts.len());
            drafts.push(Draft {
                source,
                target,
                amount,
                status: x.verification_status,
                transactions: vec![t.clone()],
            });
        }
    }

    let depth = sub_columns(&nodes, &drafts);
    let mut keys: Vec<(usize, usize)> = Vec::new();
    for (i, n) in nodes.iter().enumerate() {
        let key = (n.band, depth[i]);
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys.sort_unstable();
    for (i, n) in nodes.iter_mut().enumerate() {
        n.column = keys
            .iter()
            .position(|k| *k == (n.band, depth[i]))
            .unwrap_or(0);
    }

    let by_column = order_rows(&mut nodes, &drafts, keys.len());
    let content_h = place(&mut nodes, &by_column);
    let edges = lay_out_edges(&nodes, drafts);
    let columns = keys
        .iter()
        .enumerate()
        .map(|(ci, (band, d))| Column {
            x: column_x(ci),
            label: (*d == 0).then(|| BANDS[*band].0),
        })
        .collect();
    #[expect(clippy::cast_precision_loss, reason = "a handful of columns")]
    let count = keys.len() as f64;
    FlowGraph {
        width: PAD_X * 2.0 + count * NODE_W + (count - 1.0) * COL_GAP,
        height: TOP + content_h + PAD_BOTTOM,
        nodes,
        edges,
        columns,
        totals,
    }
}

#[expect(clippy::cast_precision_loss, reason = "a handful of columns")]
fn column_x(ci: usize) -> f64 {
    PAD_X + ci as f64 * (NODE_W + COL_GAP)
}

/// Within a band, a node fed by another of the same band moves one sub-column right
/// (`flow-graph.ts:271-287`).
fn sub_columns(nodes: &[GraphNode], drafts: &[Draft]) -> Vec<usize> {
    let mut depth = vec![0_usize; nodes.len()];
    for _ in 0..nodes.len() {
        let mut changed = false;
        for e in drafts {
            if nodes[e.source].band != nodes[e.target].band {
                continue;
            }
            let want = depth[e.source] + 1;
            if want > depth[e.target] && want < nodes.len() {
                depth[e.target] = want;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    depth
}

/// Orders each column's rows (`flow-graph.ts:300-327`): the first column by amount, later ones
/// by the amount-weighted average row of what feeds them, then by the larger side, then by name;
/// unassigned at the foot. Returns the node indexes of each column, top to bottom.
fn order_rows(nodes: &mut [GraphNode], drafts: &[Draft], columns: usize) -> Vec<Vec<usize>> {
    let mut by_column: Vec<Vec<usize>> = vec![Vec::new(); columns];
    for (i, n) in nodes.iter().enumerate() {
        by_column[n.column].push(i);
    }
    let mut row_of: HashMap<usize, usize> = HashMap::new();
    for (ci, col) in by_column.iter_mut().enumerate() {
        let score = |n: usize| -> f64 {
            let feeds: Vec<&Draft> = drafts
                .iter()
                .filter(|e| e.target == n && row_of.contains_key(&e.source))
                .collect();
            if ci == 0 || feeds.is_empty() {
                return f64::INFINITY;
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "JS numbers, as the web sums them"
            )]
            let (weight, sum) = feeds.iter().fold((0.0, 0.0), |(w, s), e| {
                let row = row_of.get(&e.source).copied().unwrap_or(0) as f64;
                (w + e.amount as f64, s + row * e.amount as f64)
            });
            sum / if weight == 0.0 { 1.0 } else { weight }
        };
        let scores: HashMap<usize, f64> = col.iter().map(|&n| (n, score(n))).collect();
        let ns: &[GraphNode] = nodes;
        col.sort_by(|&a, &b| {
            let (sa, sb) = (scores[&a], scores[&b]);
            // `score(a) - score(b)` is NaN for two infinities, which reads as a tie.
            let by_score = if sa.is_infinite()
                && sb.is_infinite()
                && sa.is_sign_positive() == sb.is_sign_positive()
            {
                Ordering::Equal
            } else {
                sa.partial_cmp(&sb).unwrap_or(Ordering::Equal)
            };
            let side = |n: &GraphNode| n.inflow.max(n.outflow);
            by_score
                .then_with(|| side(&ns[b]).cmp(&side(&ns[a])))
                .then_with(|| ns[a].name.cmp(&ns[b].name))
        });
        col.sort_by_key(|&n| ns[n].node_type == MapNodeType::Unassigned);
        for (row, &n) in col.iter().enumerate() {
            row_of.insert(n, row);
        }
    }
    by_column
}

/// Every column hangs from the same top line (`flow-graph.ts:329-342`). Returns the tallest
/// column's content height.
fn place(nodes: &mut [GraphNode], by_column: &[Vec<usize>]) -> f64 {
    let mut content_h: f64 = 0.0;
    for (ci, col) in by_column.iter().enumerate() {
        let mut y = TOP;
        for &i in col {
            let n = &mut nodes[i];
            n.lines = wrap_name(&n.name);
            n.h = node_height(n.lines.len());
            n.x = column_x(ci);
            n.y = y;
            y += n.h + ROW_GAP;
        }
        content_h = content_h.max(y - ROW_GAP - TOP);
    }
    content_h
}

/// Line weights and paths (`flow-graph.ts:344-409`).
fn lay_out_edges(nodes: &[GraphNode], drafts: Vec<Draft>) -> Vec<GraphEdge> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "JS numbers, as the web divides them"
    )]
    let max_amount = drafts.iter().map(|e| e.amount as f64).fold(1.0, f64::max);
    #[expect(clippy::cast_precision_loss, reason = "JS numbers")]
    let width_of =
        |a: Minor| MIN_STROKE + (MAX_STROKE - MIN_STROKE) * (a as f64 / max_amount).sqrt();
    let ports = |by_source: bool| -> Vec<f64> {
        let mut out = vec![0.0; drafts.len()];
        let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
        for (i, e) in drafts.iter().enumerate() {
            let owner = if by_source { e.source } else { e.target };
            match groups.iter_mut().find(|(o, _)| *o == owner) {
                Some((_, g)) => g.push(i),
                None => groups.push((owner, vec![i])),
            }
        }
        for (owner, mut group) in groups {
            let other = |i: usize| {
                let e = &drafts[i];
                nodes[if by_source { e.target } else { e.source }].y
            };
            group.sort_by(|&a, &b| other(a).total_cmp(&other(b)));
            let n = &nodes[owner];
            #[expect(clippy::cast_precision_loss, reason = "a handful of edges")]
            let count = group.len() as f64 + 1.0;
            for (k, &i) in group.iter().enumerate() {
                #[expect(clippy::cast_precision_loss, reason = "a handful of edges")]
                let k = k as f64 + 1.0;
                out[i] = n.y + (n.h * k) / count;
            }
        }
        out
    };
    let out_ports = ports(true);
    let in_ports = ports(false);
    drafts
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let (s, t) = (&nodes[e.source], &nodes[e.target]);
            let (x1, y1, x2, y2) = (s.x + NODE_W, out_ports[i], t.x, in_ports[i]);
            let mx = f64::midpoint(x1, x2);
            GraphEdge {
                id: format!("{}->{}", s.id, t.id),
                source: s.id.clone(),
                target: t.id.clone(),
                amount: e.amount,
                status: e.status,
                transactions: e.transactions,
                width: width_of(e.amount),
                path: EdgePath { x1, y1, mx, x2, y2 },
                mid: (mx, f64::midpoint(y1, y2)),
            }
        })
        .collect()
}

/// What is selected on the map (`Selection`, `flow-graph.ts:457`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Selection {
    /// A node, by id.
    Node(String),
    /// An edge, by id.
    Edge(String),
}

/// The nodes and edges upstream and downstream of a selection, in the order found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Connected {
    /// Node ids.
    pub nodes: Vec<String>,
    /// Edge ids.
    pub edges: Vec<String>,
}

impl Connected {
    fn add_node(&mut self, id: &str) {
        if !self.nodes.iter().any(|n| n == id) {
            self.nodes.push(id.to_owned());
        }
    }

    fn add_edge(&mut self, id: &str) {
        if !self.edges.iter().any(|e| e == id) {
            self.edges.push(id.to_owned());
        }
    }

    /// Whether the node is lit.
    #[must_use]
    pub fn has_node(&self, id: &str) -> bool {
        self.nodes.iter().any(|n| n == id)
    }

    /// Whether the edge is lit.
    #[must_use]
    pub fn has_edge(&self, id: &str) -> bool {
        self.edges.iter().any(|e| e == id)
    }
}

/// `connectedSet` (`flow-graph.ts:421-455`): breadth first up and down from a node, or up from
/// an edge's source and down from its target; none for no selection or an edge not on the map.
#[must_use]
pub fn connected_set(graph: &FlowGraph, selection: Option<&Selection>) -> Option<Connected> {
    let selection = selection?;
    let mut set = Connected::default();
    let walk = |start: &str, down: bool, set: &mut Connected| {
        let mut queue = std::collections::VecDeque::from([start.to_owned()]);
        let mut seen = vec![start.to_owned()];
        while let Some(id) = queue.pop_front() {
            set.add_node(&id);
            for e in &graph.edges {
                let (from, to) = if down {
                    (&e.source, &e.target)
                } else {
                    (&e.target, &e.source)
                };
                if *from != id {
                    continue;
                }
                set.add_edge(&e.id);
                if !seen.contains(to) {
                    seen.push(to.clone());
                    queue.push_back(to.clone());
                }
            }
        }
    };
    match selection {
        Selection::Node(id) => {
            walk(id, false, &mut set);
            walk(id, true, &mut set);
        }
        Selection::Edge(id) => {
            let edge = graph.edge(id)?;
            set.add_edge(&edge.id);
            walk(&edge.source, false, &mut set);
            walk(&edge.target, true, &mut set);
        }
    }
    Some(set)
}

// ---------------------------------------------------------------------------------------------
// The page
// ---------------------------------------------------------------------------------------------

/// `ZOOM_MIN` (`flows.tsx:36`).
pub const ZOOM_MIN: f64 = 0.4;
/// `ZOOM_MAX`.
pub const ZOOM_MAX: f64 = 1.6;
/// `ZOOM_STEP`.
pub const ZOOM_STEP: f64 = 0.1;
/// `LEGIBLE_MIN` (`flows.tsx:41`): the opening view never shrinks the map below this.
pub const LEGIBLE_MIN: f64 = 0.9;

/// The zoom the user chose (`flows.tsx:49-50`): none yet (the legible fit), the whole map, or a
/// level.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Zoom {
    /// The opening view: fit, never below [`LEGIBLE_MIN`].
    #[default]
    Legible,
    /// The whole map, however small (down to [`ZOOM_MIN`]).
    Fit,
    /// A level.
    Level(f64),
}

impl Zoom {
    /// The scale drawn (`flows.tsx:96-101`) for a map container `container` wide; a pixel is
    /// left for the border so a fitted map never scrolls.
    #[must_use]
    pub fn scale(self, container: f64, graph_width: f64) -> f64 {
        let ratio = if container > 0.0 && graph_width > 0.0 {
            (container - 2.0) / graph_width
        } else {
            1.0
        };
        match self {
            Self::Fit => ratio.clamp(ZOOM_MIN, 1.0),
            Self::Legible => ratio.clamp(LEGIBLE_MIN, 1.0),
            Self::Level(z) => z,
        }
    }

    /// `stepZoom(dir)` (`flows.tsx:102-107`): one step from the scale drawn, to one decimal,
    /// within the limits.
    #[must_use]
    pub fn step(scale: f64, up: bool) -> Self {
        let dir = if up { 1.0 } else { -1.0 };
        let z = ((scale + dir * ZOOM_STEP) * 10.0 + 0.5).floor() / 10.0;
        Self::Level(z.clamp(ZOOM_MIN, ZOOM_MAX))
    }

    /// The zoom readout, `90%` (`Math.round(scale * 100)`).
    #[must_use]
    pub fn percent(scale: f64) -> String {
        format!("{}%", (scale * 100.0 + 0.5).floor())
    }
}

/// What the screen shows under its header (`flows.tsx:132-249`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// No records loaded (the web's loading state).
    Loading,
    /// No movement at all.
    Empty,
    /// None in the period: the title, the window sentence and whether "Show the last year"
    /// is offered.
    EmptyPeriod {
        /// `No movements in the last month`.
        title: String,
        /// `The month view covers ... and nothing was recorded in it.`
        body: Option<String>,
        /// `Show the last year`, when the period is not the widest.
        widen: Option<String>,
    },
    /// The map.
    Map,
}

/// "Loading movements" (`flows.tsx:134-137`).
pub const LOADING_TITLE: &str = "Loading movements";
/// Its body.
pub const LOADING_BODY: &str = "Building the flow map from your recorded transactions.";
/// "No movements recorded yet" (`flows.tsx:158-168`).
pub const EMPTY_TITLE: &str = "No movements recorded yet";
/// Its body.
pub const EMPTY_BODY: &str = "The map draws each recorded movement from donor to recipient. \
                              Record your first transaction to start it.";
/// The legend's note (`flows.tsx:193`).
pub const LEGEND_NOTE: &str = "Line weight carries the amount.";
/// The help under the map (`flows.tsx:215-220`), with the desktop's keys: a click for the
/// pointer, Tab and Esc as on the web.
pub const HELP: &str = "Select a donor, account or recipient to isolate everything upstream and \
                        downstream of it; select a line to see the movements it carries. Tab \
                        moves through the map in money order; Esc clears the selection.";
/// `Where money came from, ...` (`flows.tsx:261`).
pub const LEDE_NONE: &str =
    "Where money came from, where it went, and the proof behind each movement.";

/// Everything the Flows screen shows for one period, derived from the records.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowPage {
    /// All movements, in `getTransactions` order.
    pub all: Vec<TransactionWithNodes>,
    /// The period.
    pub period: PeriodId,
    /// Its window.
    pub window: Option<Window>,
    /// The map of the movements in it.
    pub graph: FlowGraph,
}

impl FlowPage {
    /// The page of `t` for `period`; `None` records read as loading.
    #[must_use]
    pub fn of(t: &OrgTransparency, period: PeriodId) -> Self {
        let all = crate::dashboard::transactions(t);
        let window = period_window(&all, period);
        let graph = build_flow_graph(&filter_by_period(&all, period));
        Self {
            all,
            period,
            window,
            graph,
        }
    }

    /// The line under the title (`flows.tsx:257-262`).
    #[must_use]
    pub fn lede(&self) -> String {
        match &self.window {
            Some(w) => format!(
                "Movements from {} to {}, counted back from the latest recorded transaction.",
                format_date(w.start.as_str()),
                format_date(w.end.as_str())
            ),
            None => LEDE_NONE.to_owned(),
        }
    }

    /// Whether the Period control shows (`all.length > 0`).
    #[must_use]
    pub fn has_movements(&self) -> bool {
        !self.all.is_empty()
    }

    /// The body (`flows.tsx:132-249`), loading aside.
    #[must_use]
    pub fn body(&self) -> Body {
        if self.all.is_empty() {
            return Body::Empty;
        }
        if !self.graph.edges.is_empty() {
            return Body::Map;
        }
        let p = self.period.period();
        let widest = WIDEST_PERIOD.period();
        Body::EmptyPeriod {
            title: format!("No movements in the {}", p.phrase),
            body: self.window.as_ref().map(|w| {
                format!(
                    "The {} view covers {} to {}, and nothing was recorded in it.",
                    p.label.to_lowercase(),
                    format_date(w.start.as_str()),
                    format_date(w.end.as_str())
                )
            }),
            widen: (self.period != WIDEST_PERIOD).then(|| format!("Show the {}", widest.phrase)),
        }
    }

    /// The key figures over the map (`flows.tsx:267-302`): Total moved, Nodes, Links, Share
    /// evidenced, each with its detail line.
    #[must_use]
    pub fn figures(&self) -> Vec<Figure> {
        let t = self.graph.totals;
        #[expect(clippy::cast_precision_loss, reason = "a share, as the web divides")]
        let share = if t.moved > 0 {
            ((t.evidenced as f64 / t.moved as f64) * 100.0 + 0.5).floor()
        } else {
            0.0
        };
        vec![
            Figure {
                value: format!("KES {}", format_compact(t.moved)),
                label: "Total moved".to_owned(),
                detail: Some(format_money(t.moved, "KES")),
            },
            Figure {
                value: self.graph.nodes.len().to_string(),
                label: "Nodes".to_owned(),
                detail: None,
            },
            Figure {
                value: self.graph.edges.len().to_string(),
                label: "Links".to_owned(),
                detail: Some(format!("{} movements", t.movements)),
            },
            Figure {
                value: format!("{share}%"),
                label: "Share evidenced".to_owned(),
                detail: Some(format!("{} has proof", format_money(t.evidenced, "KES"))),
            },
        ]
    }
}

/// The node box's amount (`flow-map.tsx:131-133, 196-201`): the side that carried more,
/// `out KES 2M` or `in KES 850K`.
#[must_use]
pub fn node_amount(node: &GraphNode) -> (&'static str, String) {
    if node.outflow > node.inflow {
        ("out KES ", format_compact(node.outflow))
    } else {
        ("in KES ", format_compact(node.inflow))
    }
}

/// A dash pattern scaled with the line (`scaleDash`, `flow-map.tsx:17-24`), so thick lines still
/// read as dashed; none for a solid line. Patterns are `proofStroke`'s
/// (`components/report/status-line.tsx:16-27`).
#[must_use]
pub fn edge_dash(status: TransactionStatus, width: f64) -> Option<[f64; 2]> {
    let dash = match status {
        TransactionStatus::Verified => return None,
        TransactionStatus::Pending => [6.0, 5.0],
        TransactionStatus::Flagged => [14.0, 8.0],
        TransactionStatus::Rejected => [2.0, 6.0],
    };
    let k = f64::max(1.0, width / 3.0);
    Some([dash[0] * k, dash[1] * k])
}

/// The flagged cross's half size (`flow-map.tsx:226`).
#[must_use]
pub fn cross_size(width: f64) -> f64 {
    f64::max(5.0, width / 2.0 + 3.0)
}

/// Where a ledger link in the details panel goes: the General ledger with the web's `source` or
/// `node` filter (`flow-details.tsx:85-93, 145-153, 222-231`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerLink {
    /// The lines of one movement, by transaction id.
    Source(String),
    /// The lines recorded against one flow node.
    Node(String),
}

impl LedgerLink {
    /// The General ledger filter this link opens (`flow-details.tsx` links to
    /// `/ledger/general?source=` or `?node=`, replacing any other filter): a movement's lines
    /// through the journal entries it is linked to (`journal_entry_ids`), else through the
    /// entries it is the source of; a node's lines through `flow_node_id`.
    #[must_use]
    pub fn filter(&self, t: Option<&OrgTransparency>) -> GlFilter {
        match self {
            Self::Source(id) => {
                let entries = t
                    .and_then(|t| t.transactions.iter().find(|x| &x.id == id))
                    .map(|x| {
                        x.journal_entry_ids
                            .iter()
                            .map(|e| EntryId::from(e.as_str()))
                            .collect()
                    })
                    .unwrap_or_default();
                GlFilter {
                    source: Some(GlSource {
                        id: id.clone(),
                        entries,
                    }),
                    ..GlFilter::default()
                }
            }
            Self::Node(id) => GlFilter {
                node: Some(FlowNodeId::from(id.as_str())),
                ..GlFilter::default()
            },
        }
    }
}

/// One movement in a details list (`MovementList`, `flow-details.tsx:175-237`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementItem {
    /// Transaction id.
    pub id: String,
    /// `from` or `to` before the counterparty; none when the description leads.
    pub direction: Option<&'static str>,
    /// The counterparty, or the description when there is none.
    pub title: String,
    /// The description under a counterparty.
    pub description: Option<String>,
    /// `KES 2,000,000`.
    pub amount: String,
    /// `18 Jan 2026`.
    pub date: String,
    /// Proof state.
    pub status: TransactionStatus,
    /// Its `Ledger lines` link, on an edge's list of several.
    pub ledger: Option<LedgerLink>,
}

/// A titled list of movements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementList {
    /// `Movements`, `Money in (3)`.
    pub title: String,
    /// The rows, newest first.
    pub items: Vec<MovementItem>,
}

/// The details panel (`FlowDetails`, `flow-details.tsx:22-170`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Details {
    /// The heading: a node's name, or `source → target`.
    pub heading: String,
    /// `3 movements on this line`, or the node's type (with the unassigned note).
    pub subtitle: String,
    /// The figures: `Moved in period` for an edge, `Received` and `Sent` for a node.
    pub figures: Vec<(&'static str, String)>,
    /// An edge's worst proof state.
    pub status: Option<TransactionStatus>,
    /// The panel's own ledger link and its label.
    pub ledger: Option<(String, LedgerLink)>,
    /// The movement lists.
    pub lists: Vec<MovementList>,
}

/// The close button's label (`flow-details.tsx:37-45`).
pub const CLEAR_LABEL: &str = "Clear selection (Esc)";

/// `byDateDesc` (`flow-details.tsx:13-15`), stable.
fn by_date_desc(list: &mut [TransactionWithNodes]) {
    list.sort_by(|a, b| {
        b.transaction
            .transaction_date
            .cmp(&a.transaction.transaction_date)
    });
}

/// `plural(n, word)` (`flow-details.tsx:17-19`).
fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn item(
    t: &TransactionWithNodes,
    direction: Option<&'static str>,
    who: Option<String>,
    ledger: bool,
) -> MovementItem {
    let x = &t.transaction;
    let (title, description) = match who {
        Some(w) => (w, Some(x.description.clone())),
        None => (x.description.clone(), None),
    };
    MovementItem {
        id: x.id.clone(),
        direction: description.as_ref().and(direction),
        title,
        description,
        amount: format_money(x.amount, "KES"),
        date: format_date(&x.transaction_date),
        status: x.verification_status,
        ledger: ledger.then(|| LedgerLink::Source(x.id.clone())),
    }
}

impl Details {
    /// The panel for `selection` on `graph`; none when it is not on the map.
    #[must_use]
    pub fn of(graph: &FlowGraph, selection: &Selection) -> Option<Self> {
        match selection {
            Selection::Edge(id) => Some(Self::edge(graph, graph.edge(id)?)),
            Selection::Node(id) => Some(Self::node(graph, graph.node(id)?)),
        }
    }

    fn edge(graph: &FlowGraph, edge: &GraphEdge) -> Self {
        let name = |id: &str| graph.node(id).map_or("Unknown", |n| n.name.as_str());
        let mut txns = edge.transactions.clone();
        by_date_desc(&mut txns);
        let several = txns.len() > 1;
        Self {
            heading: format!("{} \u{2192} {}", name(&edge.source), name(&edge.target)),
            subtitle: format!("{} on this line", plural(txns.len(), "movement")),
            figures: vec![("Moved in period", format_money(edge.amount, "KES"))],
            status: Some(edge.status),
            ledger: (txns.len() == 1).then(|| {
                (
                    "Ledger lines for this movement".to_owned(),
                    LedgerLink::Source(txns[0].transaction.id.clone()),
                )
            }),
            lists: vec![MovementList {
                title: "Movements".to_owned(),
                items: txns.iter().map(|t| item(t, None, None, several)).collect(),
            }],
        }
    }

    fn node(graph: &FlowGraph, node: &GraphNode) -> Self {
        let collect = |into: bool| {
            let mut list: Vec<TransactionWithNodes> = graph
                .edges
                .iter()
                .filter(|e| {
                    if into {
                        e.target == node.id
                    } else {
                        e.source == node.id
                    }
                })
                .flat_map(|e| e.transactions.iter().cloned())
                .collect();
            by_date_desc(&mut list);
            list
        };
        let (incoming, outgoing) = (collect(true), collect(false));
        let unassigned = node.id == UNASSIGNED_ID;
        let mut lists = Vec::new();
        if !incoming.is_empty() {
            lists.push(MovementList {
                title: format!("Money in ({})", incoming.len()),
                items: incoming
                    .iter()
                    .map(|t| {
                        item(
                            t,
                            Some("from"),
                            t.from_node.as_ref().map(|n| n.name.clone()),
                            false,
                        )
                    })
                    .collect(),
            });
        }
        if !outgoing.is_empty() {
            lists.push(MovementList {
                title: format!("Money out ({})", outgoing.len()),
                items: outgoing
                    .iter()
                    .map(|t| {
                        let who = t
                            .to_node
                            .as_ref()
                            .map_or_else(|| UNASSIGNED_NAME.to_owned(), |n| n.name.clone());
                        item(t, Some("to"), Some(who), false)
                    })
                    .collect(),
            });
        }
        Self {
            heading: node.name.clone(),
            subtitle: if unassigned {
                format!(
                    "{}. These movements left an account without a recipient on record.",
                    node.node_type.label()
                )
            } else {
                node.node_type.label().to_owned()
            },
            figures: vec![
                ("Received", format_money(node.inflow, "KES")),
                ("Sent", format_money(node.outflow, "KES")),
            ],
            status: None,
            ledger: (!unassigned).then(|| {
                (
                    format!("Ledger lines for {}", node.name),
                    LedgerLink::Node(node.id.clone()),
                )
            }),
            lists,
        }
    }
}

/// The proof label of an edge's status, lower case, for its accessible name (`flow-map.tsx:224`).
#[must_use]
pub fn status_word(status: TransactionStatus) -> String {
    proof_label(status).to_lowercase()
}

#[cfg(test)]
mod tests;
