use super::measure::Measure;

#[derive(Clone, PartialEq)]
pub struct Score {
    pub measures: Vec<Measure>,
}
