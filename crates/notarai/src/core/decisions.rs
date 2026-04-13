use std::path::Path;

/// A decision proposal stored in `.notarai/decision-log.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionProposal {
    pub spec_path: String,
    pub date: String,
    pub choice: String,
    pub rationale: String,
    pub origin: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<String>,
}

/// The decision log: a list of proposals persisted as JSON.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionLog {
    pub proposals: Vec<DecisionProposal>,
}

impl DecisionLog {
    pub fn log_path(project_root: &Path) -> std::path::PathBuf {
        project_root.join(".notarai/decision-log.json")
    }

    /// Load the decision log from disk. Returns an empty log if the file
    /// does not exist or cannot be parsed.
    pub fn load(project_root: &Path) -> Self {
        let path = Self::log_path(project_root);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return DecisionLog { proposals: vec![] },
        };
        serde_json::from_str(&content).unwrap_or(DecisionLog { proposals: vec![] })
    }

    /// Save the decision log to disk.
    pub fn save(&self, project_root: &Path) -> Result<(), String> {
        let path = Self::log_path(project_root);
        let json =
            serde_json::to_string_pretty(&self).map_err(|e| format!("JSON serialize: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("write {}: {e}", path.display()))
    }

    /// Return proposals filtered by status. Pass None to return all.
    pub fn filter_by_status(&self, status: Option<&str>) -> Vec<(usize, &DecisionProposal)> {
        self.proposals
            .iter()
            .enumerate()
            .filter(|(_, p)| status.is_none_or(|s| p.status == s))
            .collect()
    }

    /// Accept a proposal: remove it from the log, append { date, choice, rationale }
    /// to the spec's YAML `decisions` array, validate the spec afterward.
    pub fn accept(
        &mut self,
        index: usize,
        project_root: &Path,
    ) -> Result<DecisionProposal, String> {
        if index >= self.proposals.len() {
            return Err(format!(
                "Index {index} out of range (log has {} entries)",
                self.proposals.len()
            ));
        }
        let proposal = &self.proposals[index];
        if proposal.status != "proposed" {
            return Err(format!(
                "Cannot accept: decision is already '{}'",
                proposal.status
            ));
        }

        let spec_abs = project_root.join(&proposal.spec_path);
        append_decision_to_spec(
            &spec_abs,
            &proposal.date,
            &proposal.choice,
            &proposal.rationale,
        )?;

        // Validate the spec after modification.
        let content = std::fs::read_to_string(&spec_abs).map_err(|e| format!("read spec: {e}"))?;
        let result = crate::core::validator::validate_spec(&content);
        if !result.valid {
            return Err(format!(
                "Spec validation failed after accepting decision: {}",
                result.errors.join("; ")
            ));
        }

        // Mark accepted and return a copy.
        let mut accepted = self.proposals.remove(index);
        accepted.status = "accepted".to_string();
        Ok(accepted)
    }

    /// Reject a proposal: mark it rejected in the log with an optional reason.
    pub fn reject(
        &mut self,
        index: usize,
        reason: Option<String>,
    ) -> Result<DecisionProposal, String> {
        if index >= self.proposals.len() {
            return Err(format!(
                "Index {index} out of range (log has {} entries)",
                self.proposals.len()
            ));
        }
        let proposal = &self.proposals[index];
        if proposal.status != "proposed" {
            return Err(format!(
                "Cannot reject: decision is already '{}'",
                proposal.status
            ));
        }
        self.proposals[index].status = "rejected".to_string();
        self.proposals[index].reject_reason = reason;
        Ok(self.proposals[index].clone())
    }
}

/// Append a decision entry to a spec file's `decisions` array.
fn append_decision_to_spec(
    spec_path: &Path,
    date: &str,
    choice: &str,
    rationale: &str,
) -> Result<(), String> {
    let content = std::fs::read_to_string(spec_path)
        .map_err(|e| format!("read {}: {e}", spec_path.display()))?;

    // Build the decision YAML block to append.
    let decision_yaml = format!(
        "\n  - date: '{}'\n    choice: '{}'\n    rationale: >\n      {}\n",
        date,
        choice.replace('\'', "''"),
        rationale.replace('\n', "\n      ")
    );

    // Find or create the decisions section.
    let new_content = if content.contains("\ndecisions:") || content.starts_with("decisions:") {
        // Append to existing decisions array.
        // Find the end of the decisions block (next top-level key or EOF).
        let decisions_start = content.find("\ndecisions:").map(|i| i + 1).unwrap_or(0);
        let after_decisions = &content[decisions_start + "decisions:".len()..];

        // Find where the decisions array ends: the next line that starts a
        // top-level key (no leading whitespace, not a comment, not empty).
        let mut insert_pos = content.len();
        let mut offset = decisions_start + "decisions:".len();
        for line in after_decisions.lines() {
            let next_offset = offset + line.len() + 1; // +1 for newline
            if !line.is_empty()
                && !line.starts_with(' ')
                && !line.starts_with('-')
                && !line.starts_with('#')
            {
                insert_pos = offset;
                break;
            }
            offset = next_offset;
        }

        let mut result = String::with_capacity(content.len() + decision_yaml.len());
        result.push_str(&content[..insert_pos]);
        // Ensure we end with a newline before the new entry.
        if !result.ends_with('\n') {
            result.push('\n');
        }
        // Remove the leading newline from decision_yaml since we just ensured one.
        result.push_str(decision_yaml.trim_start_matches('\n'));
        result.push_str(&content[insert_pos..]);
        result
    } else {
        // No decisions section exists. Add one before artifacts (or at EOF).
        let insert_before = content
            .find("\nartifacts:")
            .or_else(|| content.find("\nconstraints:"))
            .or_else(|| content.find("\ninvariants:"));

        match insert_before {
            Some(pos) => {
                let mut result = String::with_capacity(content.len() + decision_yaml.len() + 20);
                result.push_str(&content[..pos]);
                result.push_str("\n\ndecisions:");
                result.push_str(&decision_yaml);
                result.push_str(&content[pos..]);
                result
            }
            None => {
                let mut result = content.clone();
                if !result.ends_with('\n') {
                    result.push('\n');
                }
                result.push_str("\ndecisions:");
                result.push_str(&decision_yaml);
                result
            }
        }
    };

    std::fs::write(spec_path, new_content)
        .map_err(|e| format!("write {}: {e}", spec_path.display()))
}
