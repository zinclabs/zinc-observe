//! Defines intermediate data types for translating between service layer data
//! structures and the database.

use std::num::TryFromIntError;

use config::meta::dashboards::reports::{
    ReportDashboardVariable as MetaReportDashboardVariable,
    ReportDestination as MetaReportDestination, ReportFrequency as MetaReportFrequency,
    ReportFrequencyType as MetaReportFrequencyType, ReportTimerange as MetaReportTimeRange,
    ReportTimerangeType as MetaReportTimeRangeType,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ReportDestinations(pub Vec<ReportDestination>);

impl From<ReportDestinations> for Vec<MetaReportDestination> {
    fn from(value: ReportDestinations) -> Self {
        value.0.into_iter().map(|d| d.into()).collect()
    }
}

impl From<Vec<MetaReportDestination>> for ReportDestinations {
    fn from(value: Vec<MetaReportDestination>) -> Self {
        Self(value.into_iter().map(|d| d.into()).collect())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportDestination {
    Email(String),
}

impl From<ReportDestination> for MetaReportDestination {
    fn from(value: ReportDestination) -> Self {
        match value {
            ReportDestination::Email(email) => Self::Email(email),
        }
    }
}

impl From<MetaReportDestination> for ReportDestination {
    fn from(value: MetaReportDestination) -> Self {
        match value {
            MetaReportDestination::Email(email) => Self::Email(email),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFrequency {
    Once,
    HourlyInterval(u32),
    DailyInterval(u32),
    WeeklyInterval(u32),
    MonthlyInterval(u32),
    Cron(String),
}

impl TryFrom<MetaReportFrequency> for ReportFrequency {
    type Error = TryFromIntError;

    fn try_from(value: MetaReportFrequency) -> Result<Self, Self::Error> {
        let freq = match value.frequency_type {
            MetaReportFrequencyType::Once => Self::Once,
            MetaReportFrequencyType::Hours => Self::HourlyInterval(value.interval.try_into()?),
            MetaReportFrequencyType::Days => Self::DailyInterval(value.interval.try_into()?),
            MetaReportFrequencyType::Weeks => Self::WeeklyInterval(value.interval.try_into()?),
            MetaReportFrequencyType::Months => Self::MonthlyInterval(value.interval.try_into()?),
            MetaReportFrequencyType::Cron => Self::Cron(value.cron),
        };
        Ok(freq)
    }
}

impl From<ReportFrequency> for MetaReportFrequency {
    fn from(value: ReportFrequency) -> Self {
        match value {
            ReportFrequency::Once => Self {
                frequency_type: MetaReportFrequencyType::Once,
                interval: 0,
                cron: "".to_string(),
            },
            ReportFrequency::HourlyInterval(interval) => Self {
                frequency_type: MetaReportFrequencyType::Hours,
                interval: interval.into(),
                cron: "".to_string(),
            },
            ReportFrequency::DailyInterval(interval) => Self {
                frequency_type: MetaReportFrequencyType::Days,
                interval: interval.into(),
                cron: "".to_string(),
            },
            ReportFrequency::WeeklyInterval(interval) => Self {
                frequency_type: MetaReportFrequencyType::Weeks,
                interval: interval.into(),
                cron: "".to_string(),
            },
            ReportFrequency::MonthlyInterval(interval) => Self {
                frequency_type: MetaReportFrequencyType::Months,
                interval: interval.into(),
                cron: "".to_string(),
            },
            ReportFrequency::Cron(expr) => Self {
                frequency_type: MetaReportFrequencyType::Cron,
                interval: 0,
                cron: expr,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TabNames(pub Vec<String>);

impl From<TabNames> for Vec<String> {
    fn from(value: TabNames) -> Self {
        value.0
    }
}

impl From<Vec<String>> for TabNames {
    fn from(value: Vec<String>) -> Self {
        Self(value)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReportDashboardVariables(pub Vec<ReportDashboardVariable>);

impl From<ReportDashboardVariables> for Vec<MetaReportDashboardVariable> {
    fn from(value: ReportDashboardVariables) -> Self {
        value.0.into_iter().map(|v| v.into()).collect()
    }
}

impl From<Vec<MetaReportDashboardVariable>> for ReportDashboardVariables {
    fn from(value: Vec<MetaReportDashboardVariable>) -> Self {
        Self(value.into_iter().map(|v| v.into()).collect())
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ReportDashboardVariable {
    pub key: String,
    pub value: String,
    pub id: Option<String>,
}

impl From<ReportDashboardVariable> for MetaReportDashboardVariable {
    fn from(value: ReportDashboardVariable) -> Self {
        Self {
            key: value.key,
            value: value.value,
            id: value.id,
        }
    }
}

impl From<MetaReportDashboardVariable> for ReportDashboardVariable {
    fn from(value: MetaReportDashboardVariable) -> Self {
        Self {
            key: value.key,
            value: value.value,
            id: value.id,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportTimerange {
    Relative { period: String },
    Absolute { from: i64, to: i64 },
}

impl From<ReportTimerange> for MetaReportTimeRange {
    fn from(value: ReportTimerange) -> Self {
        match value {
            ReportTimerange::Relative { period } => Self {
                range_type: MetaReportTimeRangeType::Relative,
                period,
                from: 0,
                to: 0,
            },
            ReportTimerange::Absolute { from, to } => Self {
                range_type: MetaReportTimeRangeType::Absolute,
                period: "".to_string(),
                from,
                to,
            },
        }
    }
}

impl From<MetaReportTimeRange> for ReportTimerange {
    fn from(value: MetaReportTimeRange) -> Self {
        match value.range_type {
            MetaReportTimeRangeType::Relative => Self::Relative {
                period: value.period,
            },
            MetaReportTimeRangeType::Absolute => Self::Absolute {
                from: value.from,
                to: value.to,
            },
        }
    }
}
