use crate::calculator::calculate;
use crate::db::Db;
use crate::models::Factory;

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub severity: Severity,
    pub factory_id: String,
    pub machine_group_id: String,
    pub message: String,
}

/// Analyse a factory and return any detected issues.
pub fn analyse_factory(db: &Db, factory: &Factory) -> Vec<Issue> {
    let mut issues = Vec::new();

    if !factory.active {
        return issues;
    }

    for group in &factory.machines {
        let recipe = match db.recipe(&group.recipe) {
            Some(r) => r,
            None => {
                issues.push(Issue {
                    severity: Severity::Critical,
                    factory_id: factory.id.clone(),
                    machine_group_id: group.id.clone(),
                    message: format!("Unknown recipe '{}'", group.recipe),
                });
                continue;
            }
        };

        let machine = match db.machine(&group.machine) {
            Some(m) => m,
            None => {
                issues.push(Issue {
                    severity: Severity::Critical,
                    factory_id: factory.id.clone(),
                    machine_group_id: group.id.clone(),
                    message: format!("Unknown machine '{}'", group.machine),
                });
                continue;
            }
        };

        // Identify primary output
        let primary_output = match recipe.outputs.first() {
            Some(o) => &o.item,
            None => continue,
        };

        let base_rate = recipe.output_rate(primary_output);
        let effective_rate = base_rate * group.clock_speed * group.count as f64;

        // Idle check
        if group.clock_speed <= 0.0 || group.count == 0 {
            issues.push(Issue {
                severity: Severity::Warning,
                factory_id: factory.id.clone(),
                machine_group_id: group.id.clone(),
                message: format!(
                    "Machine group '{}' appears idle (count={}, clock={:.0}%)",
                    group.id,
                    group.count,
                    group.clock_speed * 100.0
                ),
            });
            continue;
        }

        // Compare against declared outputs
        for factory_out in &factory.outputs {
            if factory_out.item == *primary_output {
                let declared = factory_out.rate;
                let ratio = if declared > 0.0 {
                    effective_rate / declared
                } else {
                    1.0
                };
                if ratio < 0.95 {
                    issues.push(Issue {
                        severity: Severity::Warning,
                        factory_id: factory.id.clone(),
                        machine_group_id: group.id.clone(),
                        message: format!(
                            "'{}' produces {:.1}/min but declares {:.1}/min for {} ({:.0}%)",
                            group.id,
                            effective_rate,
                            declared,
                            primary_output,
                            ratio * 100.0
                        ),
                    });
                } else if ratio > 1.05 {
                    issues.push(Issue {
                        severity: Severity::Info,
                        factory_id: factory.id.clone(),
                        machine_group_id: group.id.clone(),
                        message: format!(
                            "'{}' produces {:.1}/min, exceeds declared {:.1}/min for {} ({:.0}%)",
                            group.id,
                            effective_rate,
                            declared,
                            primary_output,
                            ratio * 100.0
                        ),
                    });
                }
            }
        }

        // Check input supply vs required
        let _ = calculate(
            recipe,
            primary_output,
            effective_rate,
            group.clock_speed,
            machine.power_mw,
        );
        for recipe_inp in &recipe.inputs {
            let required = (recipe_inp.amount as f64 / recipe.cycle_time_s)
                * 60.0
                * group.clock_speed
                * group.count as f64;

            let supplied: f64 = factory
                .inputs
                .iter()
                .filter(|fi| fi.item == recipe_inp.item)
                .map(|fi| fi.rate)
                .sum();

            if supplied > 0.0 && supplied < required * 0.95 {
                issues.push(Issue {
                    severity: Severity::Critical,
                    factory_id: factory.id.clone(),
                    machine_group_id: group.id.clone(),
                    message: format!(
                        "Input '{}' undersupplied: need {:.1}/min, have {:.1}/min",
                        recipe_inp.item, required, supplied
                    ),
                });
            }
        }
    }

    issues
}
