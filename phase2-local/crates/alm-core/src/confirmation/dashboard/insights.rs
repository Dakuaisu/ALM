//! Dashboard extras (HTML L2498–2515 dashStats, L2663–2688 topCustomers/deltaChip, L3116–3150
//! lastNDays/topProducts, L3337–3360 payModeSplit) and the chart geometry of lineChart
//! (L1669–1745) and donut (L1833–1870), as numbers for a renderer to draw.
use super::{DashboardError, DashboardPeriod, DashboardWindow, windows};
use crate::confirmation::image_handoff::format_amount;
use crate::confirmation::{ConfirmationFixture, ReceiptMode, SalesPeriod};
use crate::{BillDate, DraftCustomer, Money, Role};

/// `deltaChip(cur, prev)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delta {
    New,
    Level,
    Up(u64),
    Down(u64),
}

impl Delta {
    /// `!prev` → new; |Δ%| < 0.5 → level; otherwise the rounded whole percentage.
    pub fn between(current: i64, prior: i64) -> Self {
        if prior == 0 {
            return Self::New;
        }
        let num = (i128::from(current) - i128::from(prior)) * 100;
        let den = i128::from(prior).abs();
        if 2 * num.abs() < den {
            return Self::Level;
        }
        let pct = u64::try_from((2 * num.abs() + den) / (2 * den)).unwrap_or(u64::MAX);
        if num > 0 {
            Self::Up(pct)
        } else {
            Self::Down(pct)
        }
    }

    /// The chip's class and text: `("up", "12%")`, `("flat", "new")`.
    pub fn chip(self) -> (&'static str, String) {
        match self {
            Self::New => ("flat", "new".into()),
            Self::Level => ("flat", "level".into()),
            Self::Up(pct) => ("up", format!("{pct}%")),
            Self::Down(pct) => ("dn", format!("{pct}%")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopProduct {
    pub item: String,
    pub packets: u64,
    pub amount: Money,
    /// Bar width relative to the first row, in whole percent.
    pub bar_percent: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartPoint {
    pub x: f64,
    pub y: f64,
}

/// lineChart on a fixed 560×230 canvas: four grid lines with labels, the series points, and the
/// x labels (every `ceil(n/8)`th and the last).
#[derive(Clone, Debug, PartialEq)]
pub struct LineChart {
    pub width: u32,
    pub height: u32,
    pub grid: Vec<(f64, String)>,
    pub points: Vec<ChartPoint>,
    pub baseline: f64,
    pub x_labels: Vec<(f64, String)>,
}

/// One donut arc (each under half a turn), its slice colour and legend text.
#[derive(Clone, Debug, PartialEq)]
pub struct DonutSlice {
    pub label: String,
    pub color: &'static str,
    pub arcs: Vec<(ChartPoint, ChartPoint)>,
    pub legend_y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Donut {
    pub width: u32,
    pub height: u32,
    pub center: ChartPoint,
    pub radius: f64,
    pub inner: f64,
    pub legend_x: f64,
    pub slices: Vec<DonutSlice>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DashboardInsights {
    pub sales_delta: Delta,
    pub collected_delta: Delta,
    pub packets: u64,
    pub packets_delta: Delta,
    /// `moneyR(cur.avg)`: sales per bill rounded to whole rupees.
    pub average: Money,
    /// `Math.round(collected / sales * 100)`, 0 without sales.
    pub collection_rate: u64,
    /// Owner-only: profit, its delta and `margin.toFixed(1)`; `None` when unavailable.
    pub profit: Option<(Money, Delta, String)>,
    pub top_products: Vec<TopProduct>,
    pub top_customers: Vec<(String, Money)>,
    pub trend: LineChart,
    /// `None` when the period has no sales (the HTML's "No sales in this period").
    pub pay_mix: Option<Donut>,
}

#[derive(Default)]
struct Window {
    sales: i64,
    collected: i64,
    packets: u64,
    products: Vec<(String, u64, i64)>,
    customers: Vec<(String, i64)>,
    mix: [i64; 3],
}

fn round_div(num: i128, den: i128) -> i128 {
    if den == 0 {
        return 0;
    }
    let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
    (2 * num + den).div_euclid(2 * den)
}

impl ConfirmationFixture {
    /// The Dashboard's extras for the selected period. Owner-only, like the Dashboard.
    ///
    /// # Errors
    /// Denies Managers and rejects records the Dashboard does not support.
    pub fn dashboard_insights(
        &self,
        role: Role,
        today: BillDate,
        period: DashboardPeriod,
    ) -> Result<DashboardInsights, DashboardError> {
        let board = self.dashboard(role, today, period)?;
        let [current, prior] = windows::windows(today, period)?;
        let cur = self.insight_window(current, board.current.collected)?;
        let prev = self.insight_window(prior, board.prior.collected)?;
        let bills = i128::try_from(board.current.bill_count).unwrap_or(i128::MAX);
        let average = i64::try_from(round_div(i128::from(cur.sales), bills * 100) * 100)
            .map_err(|_| DashboardError::Overflow)?;
        let collection_rate = if cur.sales == 0 {
            0
        } else {
            u64::try_from(round_div(
                i128::from(cur.collected) * 100,
                i128::from(cur.sales),
            ))
            .unwrap_or(0)
        };
        let profit_of = |window: DashboardWindow| {
            self.sales_history(
                role,
                today,
                SalesPeriod::Custom((Some(window.start), Some(window.end))),
                "",
            )
            .ok()
            .and_then(|history| history.profit.ok())
            .map(|profit| profit.profit)
        };
        let profit = match (profit_of(current), profit_of(prior)) {
            (Some(now), Some(before)) => {
                let tenths = round_div(i128::from(now.minor_units()) * 1000, i128::from(cur.sales));
                let sign = if tenths < 0 { "-" } else { "" };
                let margin = format!("{sign}{}.{}", tenths.abs() / 10, tenths.abs() % 10);
                Some((
                    now,
                    Delta::between(now.minor_units(), before.minor_units()),
                    margin,
                ))
            }
            _ => None,
        };
        let mut top_products = cur.products.clone();
        top_products.sort_by_key(|row| std::cmp::Reverse(row.1));
        top_products.truncate(6);
        let first = top_products.first().map_or(0, |row| row.1);
        let top_products = top_products
            .into_iter()
            .map(|(item, packets, amount)| TopProduct {
                bar_percent: if first == 0 {
                    0
                } else {
                    u64::try_from(round_div(i128::from(packets) * 100, i128::from(first)))
                        .unwrap_or(0)
                },
                item,
                packets,
                amount: Money::from_minor_units(amount),
            })
            .collect();
        let mut top_customers = cur.customers.clone();
        top_customers.sort_by_key(|row| std::cmp::Reverse(row.1));
        top_customers.truncate(6);
        Ok(DashboardInsights {
            sales_delta: Delta::between(cur.sales, prev.sales),
            collected_delta: Delta::between(cur.collected, prev.collected),
            packets: cur.packets,
            packets_delta: Delta::between(
                i64::try_from(cur.packets).unwrap_or(i64::MAX),
                i64::try_from(prev.packets).unwrap_or(i64::MAX),
            ),
            average: Money::from_minor_units(average),
            collection_rate,
            profit,
            top_products,
            top_customers: top_customers
                .into_iter()
                .map(|(name, total)| (name, Money::from_minor_units(total)))
                .collect(),
            trend: self.trend(today)?,
            pay_mix: donut(cur.mix),
        })
    }

    fn insight_window(
        &self,
        window: DashboardWindow,
        collected: Money,
    ) -> Result<Window, DashboardError> {
        let mut out = Window {
            collected: collected.minor_units(),
            ..Window::default()
        };
        for (id, (review, receipt, _)) in &self.current_sales {
            let cancelled = self.lifecycle.get(id).is_some_and(|(c, _)| *c);
            let Some(date) = review.draft.bill_date() else {
                continue;
            };
            if cancelled || !window.contains(date) {
                continue;
            }
            let total = receipt.total.minor_units();
            let paid = receipt
                .received
                .ok_or(DashboardError::UnsupportedRecord)?
                .minor_units();
            out.sales = out
                .sales
                .checked_add(total)
                .ok_or(DashboardError::Overflow)?;
            let name = match review.draft.customer() {
                DraftCustomer::Selected { name, .. } | DraftCustomer::Typed(name) => name.clone(),
                DraftCustomer::Unassigned => "—".to_owned(),
            };
            match out.customers.iter_mut().find(|row| row.0 == name) {
                Some(row) => row.1 = row.1.saturating_add(total),
                None => out.customers.push((name, total)),
            }
            let slot = usize::from(review.payment_mode == ReceiptMode::Online);
            if paid > 0 {
                out.mix[slot] = out.mix[slot].saturating_add(paid);
            }
            if total - paid > 50 {
                out.mix[2] = out.mix[2].saturating_add(total - paid);
            }
            for (index, line) in review.draft.lines().iter().enumerate() {
                let (quantity, price) = review
                    .line_values(index)
                    .map_err(|_| DashboardError::UnsupportedRecord)?;
                let amount = price.minor_units().saturating_mul(i64::from(quantity));
                out.packets += u64::from(quantity);
                let item = self
                    .catalog
                    .item_name(line.product_id())
                    .unwrap_or_default()
                    .to_owned();
                match out.products.iter_mut().find(|row| row.0 == item) {
                    Some(row) => {
                        row.1 += u64::from(quantity);
                        row.2 = row.2.saturating_add(amount);
                    }
                    None => out.products.push((item, u64::from(quantity), amount)),
                }
            }
        }
        Ok(out)
    }

    /// "Sales — last 14 days": bill totals per day ending today.
    fn trend(&self, today: BillDate) -> Result<LineChart, DashboardError> {
        let mut days = vec![today];
        for _ in 1..14 {
            let earlier = windows::before(*days.last().ok_or(DashboardError::DateOutOfRange)?, 1)?;
            days.push(earlier);
        }
        days.reverse();
        let totals: Vec<i64> = days
            .iter()
            .map(|day| {
                self.current_sales
                    .iter()
                    .filter(|(id, (review, _, _))| {
                        !self.lifecycle.get(*id).is_some_and(|(c, _)| *c)
                            && review.draft.bill_date() == Some(*day)
                    })
                    .map(|(_, (_, receipt, _))| receipt.total.minor_units())
                    .fold(0_i64, i64::saturating_add)
            })
            .collect();
        let labels = days
            .iter()
            .map(|day| day.display().chars().take(6).collect())
            .collect::<Vec<String>>();
        Ok(line_chart(&labels, &totals))
    }
}

/// `niceMax` over whole rupees: 1, 2, 5 or 10 × a power of ten, at least ₹1.
fn nice_max(paise: i64) -> i64 {
    let rupees = (paise.max(100) + 99) / 100;
    let mut power = 1_i64;
    while power.saturating_mul(10) <= rupees {
        power = power.saturating_mul(10);
    }
    let step = [1, 2, 5, 10]
        .into_iter()
        .find(|f| f * power >= rupees)
        .unwrap_or(10);
    step.saturating_mul(power).saturating_mul(100)
}

#[allow(clippy::cast_precision_loss)]
fn line_chart(labels: &[String], data: &[i64]) -> LineChart {
    let (width, height) = (560_u32, 230_u32);
    let (l, r, t, b) = (46.0, 12.0, 14.0, 24.0);
    let iw = f64::from(width) - l - r;
    let ih = f64::from(height) - t - b;
    let max = nice_max(data.iter().copied().max().unwrap_or(0));
    let grid = (0..=4)
        .map(|i| {
            let value = Money::from_minor_units(max / 4 * (4 - i));
            (t + ih * f64::from(i as u8) / 4.0, format_amount(value))
        })
        .collect();
    let n = data.len();
    let x = |i: usize| {
        if n <= 1 {
            l + iw / 2.0
        } else {
            l + iw * i as f64 / (n - 1) as f64
        }
    };
    let points = data
        .iter()
        .enumerate()
        .map(|(i, v)| ChartPoint {
            x: x(i),
            y: t + ih * (1.0 - *v as f64 / max as f64),
        })
        .collect();
    let step = n.div_ceil(8).max(1);
    let x_labels = labels
        .iter()
        .enumerate()
        .filter(|(i, _)| i % step == 0 || *i + 1 == n)
        .map(|(i, label)| (x(i), label.clone()))
        .collect();
    LineChart {
        width,
        height,
        grid,
        points,
        baseline: t + ih,
        x_labels,
    }
}

/// payModeSplit (`Math.round` rupees per slice, empty slices dropped) drawn as the donut.
#[allow(clippy::cast_precision_loss)]
fn donut(mix: [i64; 3]) -> Option<Donut> {
    let parts: Vec<(&str, &'static str, i64)> = [
        ("Cash", "#22c55e", mix[0]),
        ("Online", "#3b82f6", mix[1]),
        ("Credit", "#f59e0b", mix[2]),
    ]
    .into_iter()
    .map(|(label, color, paise)| {
        (
            label,
            color,
            Money::from_minor_units(paise).rounded_rupees(),
        )
    })
    .filter(|part| part.2 > 0)
    .collect();
    let total: i64 = parts.iter().map(|part| part.2).sum();
    if total <= 0 {
        return None;
    }
    let (width, height) = (560_u32, 230_u32);
    let h = f64::from(height);
    let center = ChartPoint {
        x: h / 2.0 + 6.0,
        y: h / 2.0,
    };
    let radius = h / 2.0 - 12.0;
    let at = |angle: f64| ChartPoint {
        x: center.x + radius * angle.cos(),
        y: center.y + radius * angle.sin(),
    };
    let mut angle = -std::f64::consts::FRAC_PI_2;
    let mut legend_y = center.y - parts.len() as f64 * 9.0 + 4.0;
    let mut slices = Vec::new();
    for (label, color, value) in parts {
        let sweep = value as f64 / total as f64 * std::f64::consts::TAU;
        let pieces = if sweep > std::f64::consts::PI { 2 } else { 1 };
        let arcs = (0..pieces)
            .map(|k| {
                let from = angle + sweep * f64::from(k) / f64::from(pieces);
                let to = angle + sweep * f64::from(k + 1) / f64::from(pieces);
                (at(from), at(to))
            })
            .collect();
        angle += sweep;
        let pct = round_div(i128::from(value) * 100, i128::from(total));
        slices.push(DonutSlice {
            label: format!("{label}  {pct}%"),
            color,
            arcs,
            legend_y,
        });
        legend_y += 20.0;
    }
    Some(Donut {
        width,
        height,
        center,
        radius,
        inner: radius * 0.6,
        legend_x: center.x + radius + 18.0,
        slices,
    })
}

#[cfg(test)]
mod tests {
    use super::{Delta, nice_max};

    #[test]
    fn delta_chip_follows_the_html_thresholds() {
        assert_eq!(Delta::between(5, 0), Delta::New);
        assert_eq!(Delta::between(1_004, 1_000), Delta::Level);
        assert_eq!(Delta::between(1_005, 1_000), Delta::Up(1));
        assert_eq!(Delta::between(750, 1_000), Delta::Down(25));
        assert_eq!(Delta::between(-50, -100), Delta::Up(50));
        assert_eq!(Delta::Up(12).chip(), ("up", "12%".into()));
        assert_eq!(nice_max(0), 100);
        assert_eq!(nice_max(24_100), 50_000);
        assert_eq!(nice_max(100_000), 100_000);
        assert_eq!(nice_max(100_001), 200_000);
    }
}
