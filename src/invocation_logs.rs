#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedInvocation {
    pub program_id: String,
    pub depth: usize,
    pub message: String,
    pub log_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InvocationAnalysis {
    pub deepest_failure: Option<FailedInvocation>,
    pub malformed: bool,
}

#[derive(Debug)]
struct Frame<'a> {
    program_id: &'a str,
    depth: usize,
}

pub fn analyze(logs: &[String]) -> InvocationAnalysis {
    let mut stack: Vec<Frame<'_>> = Vec::new();
    let mut result = InvocationAnalysis::default();

    for (log_index, line) in logs.iter().enumerate() {
        if let Some((program_id, depth)) = parse_invoke(line) {
            if depth != stack.len() + 1 {
                result.malformed = true;
            }
            stack.push(Frame { program_id, depth });
            continue;
        }

        if let Some(program_id) = parse_success(line) {
            pop_matching(&mut stack, program_id, &mut result.malformed);
            continue;
        }

        if let Some((program_id, message)) = parse_failure(line) {
            let depth = stack.last().map_or(0, |frame| frame.depth);
            let matches_stack = stack
                .last()
                .is_some_and(|frame| frame.program_id == program_id);
            if !matches_stack {
                result.malformed = true;
            }

            if matches_stack
                && result
                    .deepest_failure
                    .as_ref()
                    .is_none_or(|failure| depth > failure.depth)
            {
                result.deepest_failure = Some(FailedInvocation {
                    program_id: program_id.to_owned(),
                    depth,
                    message: message.to_owned(),
                    log_index,
                });
            }

            pop_matching(&mut stack, program_id, &mut result.malformed);
        }
    }

    if !stack.is_empty() {
        result.malformed = true;
    }

    result
}

fn parse_invoke(line: &str) -> Option<(&str, usize)> {
    let rest = line.strip_prefix("Program ")?;
    let (program_id, depth) = rest.rsplit_once(" invoke [")?;
    let depth = depth.strip_suffix(']')?.parse().ok()?;
    (!program_id.is_empty()).then_some((program_id, depth))
}

fn parse_success(line: &str) -> Option<&str> {
    line.strip_prefix("Program ")?.strip_suffix(" success")
}

fn parse_failure(line: &str) -> Option<(&str, &str)> {
    line.strip_prefix("Program ")?.split_once(" failed: ")
}

fn pop_matching<'a>(stack: &mut Vec<Frame<'a>>, program_id: &str, malformed: &mut bool) {
    match stack.pop() {
        Some(frame) if frame.program_id == program_id => {}
        Some(_) | None => *malformed = true,
    }
}

#[cfg(test)]
mod tests {
    use super::analyze;

    fn logs(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| (*line).to_owned()).collect()
    }

    #[test]
    fn selects_the_deepest_nested_failure() {
        let analysis = analyze(&logs(&[
            "Program outer invoke [1]",
            "Program inner invoke [2]",
            "Program inner failed: custom program error: 0x2a",
            "Program outer failed: custom program error: 0x2a",
        ]));

        assert!(!analysis.malformed);
        let failure = analysis.deepest_failure.expect("nested failure");
        assert_eq!(failure.program_id, "inner");
        assert_eq!(failure.depth, 2);
    }

    #[test]
    fn marks_mismatched_termination_as_malformed() {
        let analysis = analyze(&logs(&[
            "Program outer invoke [1]",
            "Program different failed: error",
        ]));

        assert!(analysis.malformed);
        assert!(analysis.deepest_failure.is_none());
    }
}
