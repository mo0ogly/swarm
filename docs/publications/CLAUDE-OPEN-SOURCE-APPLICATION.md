# Candidature Swarm — Claude for Open Source

Brouillon conservé le 16 octobre 2026. Aucune candidature envoyée.

Programme : [Claude for Open Source](https://claude.com/contact-sales/claude-for-oss).

## Tell us about the project's reach and impact

I maintain Swarm, a local platform for coordinating AI agents through a deterministic execution engine. It addresses a practical gap in the agent ecosystem: making multi-agent workflows accountable, inspectable, and governed by explicit rules.

The deterministic engine is part of the agentic workflow itself. Agents reason, propose decisions, and perform authorized work; the engine checks identities, permissions, dependencies, resource reservations, and required evidence before applying transitions. Planning, execution, and review have distinct responsibilities.

Agent identities make actions and decisions attributable to their responsible actors. Audit evidence is recorded in an append-only, hash-linked journal, preserving the sequence of events and supporting integrity verification. This helps users investigate failures and distinguish an agent's claim of success from a result supported by checks.

Versioned workflow templates help define reusable stages, responsibilities, dependencies, decision paths, validation requirements, and recovery procedures. A visual graph, CLI, and bilingual French/English documentation make these workflows accessible and inspectable.

The shared engine is intended for two complementary uses: software development, covering implementation, testing, and review; and business processes, covering repeatable case handling, approvals, exceptions, and controlled actions. The dedicated business-facing interface remains under development.

Swarm is early-stage, with an experimental release. I do not yet have verified large-scale adoption or downstream dependency figures. Its contribution is a concrete implementation of governed agent coordination. Program support would help strengthen Claude Code integration, validate failure and recovery paths, and improve documentation for future users and contributors.

## How will you use the subscription for your project?

I would use the subscription for hands-on development, code review, and testing of Swarm, with particular attention to its Claude Code integration.

My immediate priorities are persistent agent memory, recovery after interruptions, and the interaction between agent reasoning and Swarm's deterministic engine. I would use Claude to investigate failures, develop regression tests, and evaluate whether permissions, agent identities, workflow transitions, and append-only audit evidence behave as intended.

I would also use it to develop reusable workflow templates for software development and business processes, and improve French and English documentation, examples, and onboarding.

The subscription would support iterative work on real scenarios, including unsuccessful attempts and recovery paths. Findings would become reproducible tests, documented limitations, and repository improvements, with deterministic checks and human review used to validate AI-generated changes.

## Notes avant envoi — hors réponses au formulaire

- Le dépôt utilise actuellement [PolyForm Noncommercial](../../LICENSE). La restriction commerciale ne correspond pas à la [définition open source de l'OSI](https://opensource.org/osd). Vérifier l'admissibilité auprès du programme ; ne pas présenter cette licence comme approuvée par l'OSI.
- Ne pas ajouter de chiffres d'adoption, de téléchargements ou de dépendances sans preuve vérifiable.
- La formulation « append-only » concerne le journal de preuves d'audit ; elle ne signifie pas que tous les fichiers de logs sont immuables.
- L'usage développement et l'usage métier partagent une cible de moteur commun ; la réponse conserve la limite sur l'interface métier encore en développement.

Dépôt : [mo0ogly/swarm](https://github.com/mo0ogly/swarm).
