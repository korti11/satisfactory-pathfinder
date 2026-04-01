const fs = require('fs');
const path = require('path');

const dataDir = path.join(__dirname, 'data');
const items = JSON.parse(fs.readFileSync(path.join(dataDir, 'items.json')));
const machines = JSON.parse(fs.readFileSync(path.join(dataDir, 'machines.json')));
const recipes = JSON.parse(fs.readFileSync(path.join(dataDir, 'recipes.json')));
const resources = JSON.parse(fs.readFileSync(path.join(dataDir, 'resources.json')));

let errors = 0;
let warnings = 0;

function error(msg) { console.error('ERROR:', msg); errors++; }
function warn(msg) { console.warn('WARN:', msg); warnings++; }

const itemIds = new Set(items.map(i => i.id));
const machineIds = new Set(machines.map(m => m.id));

// No duplicate IDs in any file
['items', 'machines', 'recipes'].forEach(name => {
  const data = name === 'items' ? items : name === 'machines' ? machines : recipes;
  const ids = data.map(x => x.id);
  const seen = new Set();
  ids.forEach(id => {
    if (seen.has(id)) error(`Duplicate ID in ${name}.json: ${id}`);
    seen.add(id);
  });
});

// Recipes validation
recipes.forEach(r => {
  // Machine exists
  if (!machineIds.has(r.machine)) error(`Recipe ${r.id}: unknown machine "${r.machine}"`);

  // cycle_time_s positive (0 allowed for equipment_workshop)
  if (r.cycle_time_s < 0) error(`Recipe ${r.id}: negative cycle_time_s`);

  // All input items exist
  r.inputs.forEach(inp => {
    if (!itemIds.has(inp.item)) error(`Recipe ${r.id}: input item "${inp.item}" not in items.json`);
    if (!Number.isInteger(inp.amount) || inp.amount <= 0) error(`Recipe ${r.id}: input "${inp.item}" amount must be positive integer, got ${inp.amount}`);
  });

  // All output items exist
  r.outputs.forEach(out => {
    if (!itemIds.has(out.item)) error(`Recipe ${r.id}: output item "${out.item}" not in items.json`);
    if (!Number.isInteger(out.amount) || out.amount <= 0) error(`Recipe ${r.id}: output "${out.item}" amount must be positive integer, got ${out.amount}`);
  });

  // Has at least one output
  if (r.outputs.length === 0) warn(`Recipe ${r.id}: no outputs`);
});

// Resources reference valid items
resources.forEach(r => {
  if (!itemIds.has(r.item)) error(`resources.json: item "${r.item}" not in items.json`);
  if (r.node_count < 0) error(`resources.json: negative node_count for ${r.item} ${r.purity}`);
  if (r.max_rate_per_node <= 0) error(`resources.json: non-positive max_rate for ${r.item} ${r.purity}`);
});

console.log(`\nValidation complete: ${errors} errors, ${warnings} warnings`);
console.log(`  Items: ${items.length}`);
console.log(`  Machines: ${machines.length}`);
console.log(`  Recipes: ${recipes.length}`);
console.log(`  Resources: ${resources.length}`);
if (errors > 0) process.exit(1);
