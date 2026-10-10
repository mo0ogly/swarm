# Guided workflows to prepare a mission

In **Prepare a project**, choose **Choose a mission template**. Ten templates
cover four general structures (application improvement, reproducible defect,
user interface and new application) and six subjects: internal tool, client
portal, API and integration, inventory management, data processing and
dashboard. Read the draft in the dialog, insert it, complete the bracketed
fields and save the need. Existing text is preserved: download it or start a new
preparation before applying another template.

The versioned FR/EN catalogue is embedded from
`tools/agent-workflows/templates/preparations.json`. The CLI exposes the same
catalogue without an AI call or mission mutation:

```sh
swarm prepare templates
swarm prepare template application
swarm prepare template correction
```

JSON includes `title.fr`, `title.en`, `need.fr`, `need.en`, referenced phases and
template version. Use these fields to build a normal `prepare create --input
request.json` request with version, action, unique event_id, expected_revision
zero, title, text and method. Saving a need does not adopt a brief or validate a
plan.

## A structure is separate from a method

Workflows propose framing, research, design when relevant, planning,
implementation, review and delivery. They execute no commands, copy no private
skills from another repository and grant no permissions.

Application, interface and product drafts use analysis and planning; defect
correction uses structured diagnosis. Availability is displayed before use and
checked before an AI call. A draft can be preserved even if the method is
unavailable. Product requirements, stories and architecture are proposed steps,
not new engine methods.

The draft asks for roles, dependencies, workspaces, evidence and limits. It does
not create a team or reviewer. Existing plan validation, organisation and launch
authorisation contracts remain necessary. No budget, acceptance, publication or
merge is preauthorised.

The catalogue contains four general structures and six embedded subjects. Importing, editing and
saving custom templates are not yet available.

## Guided preparation and proposed team

Each template has eight questions: outcome or reproducible defect, audience,
scope, exclusions, constraints, observable success, verification and recovery.
**Check my answers** renders the draft and lists missing answers. Each missing
item focuses its matching field. **Use this draft** checks the answers again
before insertion. An incomplete draft can be preserved and is clearly labelled.

Answers survive template switches and closing the dialog within the current
page. To preserve them after reload, insert the draft and save the requirements
with the existing form. A failed check inserts nothing and preserves answers.

Cards describe a proposed planner, workers and independent reviewer. They do
not represent active agents. A subplanner remains conditional on justified
decomposition. Actual roles, workspaces and AI models are defined and checked
in the plan.

The deterministic check verifies presence, not relevance or truth.
`need_complete` does not make a plan valid or ready to launch.
`launch_authorized` is always false. Engine checks and authorisations still apply.

### The same check from the CLI

```sh
swarm prepare template correction
swarm prepare template-check correction --input answers.json
```

Partial answer example (no write or AI call):

```json
{
  "language": "en",
  "answers": {
    "objective": "Save does not preserve the name after reload.",
    "scope": "Preparation form only",
    "exclusions": "Preserve the graph and existing missions"
  }
}
```

Accepted answer IDs: `objective`, `audience`, `scope`, `exclusions`,
`constraints`, `acceptance`, `verification`, `recovery`. Unknown keys are
rejected. Each answer is limited to 1,000 UTF-8 bytes. The output contains the
rendered draft, `missing` answer IDs, `answered`, `total`, `need_complete`,
`proposed_team`, `recommended_method` and `launch_authorized`. A subject also
returns `intervention`. Blank answers or answers containing
the template's completion placeholders remain missing.

## Methods supplied to roles

Cards and drafts describe each role's method: Analysis and planning to frame the need, Implementation and verification to
implement and verify an authorised task, Independent review to examine
evidence and propose corrections. Subplanners retain planning boundaries. Preparation
permits no execution, even when a method also describes implementation phases.

The catalogue's `team[].workflow` and the check's `proposed_team[].workflow`
fields come from the same
`agentWorkflow` function as launches: version, role, methods and fingerprint of
the bundled framing. This describes methods supplied by the engine; it does not
prove an agent followed them. Actual launch instructions include this framing;
a method proposed in the need cannot replace it. Roles gain no permission,
budget or acceptance authority.

“Examine and improve” distinguishes Prepare, Implement, Verify and Improve.
The reviewer proposes corrections; it does not implement them. Preparation
remains limited to analysis and planning. Display names do not change technical
identifiers or bundled methods.

**Fix a reproducible defect** now uses **Diagnose and fix a problem**. During
preparation, AI proposes the diagnostic plan without tools or corrections. New
workers also receive the embedded method under their role boundaries.


## Subjects and interventions

The **Projects and development** home displays the six subjects. Each entry
shows its expected result and required evidence, then opens the selected
subject in preparation. The dialog adds an example, deliverables and risks to
examine. Selecting a subject does not save a need.

Choose **Create**, **Improve**, **Correct** or **Migrate**. The subject and
intervention enrich the draft while retaining the shared framework: **five
stages common to the product, then six stages repeated for each feature**.
Existing work is examined before reuse; non-applicable stages require a reason.
No automatic stage tracker is created.

Correct recommends diagnosis; other interventions recommend the product
workflow. The choice and answers survive subject switches in the current page.
An explicit save then persists the need and method. Migration still requires
a cutover decision, parity checks and verified recovery; choosing Migrate
grants no authorisation.

```sh
swarm prepare template service-api
swarm prepare template-check service-api --input api-answers.json
```

The answer file uses the previous contract with an optional
`"intervention": "migrate"` field. Accepted values are `create`, `improve`,
`correct`, `migrate`; omitting it defaults to `create` for a subject.
The four general structures retain their contract without an intervention.
The public `/api/v1/preparations/template-check` API uses the same projection
as the CLI. The catalogue and FR/EN metadata are versioned together.

## Client portal work model

Version 2 provides an inspectable graph and five increments: shared foundations,
sign-in and sessions, documents, access administration, and end-to-end revocation.
Foundations are shared; each feature follows the six-stage product cycle. Design
and backend tasks converge into integration, review and prepared delivery.

Select an increment, inspect its tasks and checks, and complete the answers.
**Download the adapted plan** returns the public `ActionPlan` v1 contract.
**Use this journey**, then save the new preparation, preserves the need and inserts
the plan into its editor. Draft and adopt the brief, resolve open decisions, then
validate the plan before converting it to missions. Answers never authorize
check commands or publication.

The same CLI projection accepts `increment` (`foundations`, `session`, `documents`,
`administration`, `revocation`) through `prepare template-check client-portal
--input answers.json` and returns `work_plan`. Each plan has at most eight tasks.
The engine checks local dependencies. Future stories stay unplanned; cross-increment
prerequisites require explicit decisions and accepted evidence before adoption.
Automatic synchronization across missions is not provided. Recovery descriptions
use existing corrections and plan revisions, rather than a conditional scheduler.
Executable validation policies still require authorization for the actual repository
and candidate.
