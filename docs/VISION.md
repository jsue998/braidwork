# Braidwork Vision

## Problem

Developers increasingly have access to several AI systems at the same time:
free accounts, paid subscriptions, APIs, official CLIs and local models.

Those resources are isolated.

The developer manually transfers context, decides which model should perform
each task, summarizes results, tracks usage and verifies outputs.

## Thesis

Braidwork treats every available AI account, subscription, API, CLI or local
model as a heterogeneous cognitive resource.

A project owns the canonical state.

Models receive task-specific views of that state.

Braidwork should use the least scarce resource capable of completing a task
and escalate only when verification or evidence indicates that it is needed.

## Initial vertical

Software engineering.

## Core primitives

### Task Capsule

A portable, provider-independent unit of work.

### Receipt

An auditable record of execution and verification.

## Non-goals

Braidwork is not:

- a chatbot;
- an AI model;
- a browser automation system;
- a provider gateway;
- a generic SaaS;
- an excuse to run as many agents as possible.
