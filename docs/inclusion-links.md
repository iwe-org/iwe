# Inclusion Links

When you place a markdown link on its own line, IWE treats it as a structural relationship—the current document becomes the "parent" of the linked document. This lets you organize knowledge into hierarchies without folders, and a single document can have multiple parents.

## Why Hierarchy Matters

Hierarchy is one of the most natural ways humans organize knowledge. We instinctively break complex information into nested structures:

- **Living things** are classified into kingdoms, classes, orders, and species
- **Places** nest from continents to countries to cities to neighborhoods
- **Organizations** have companies, departments, teams, and roles
- **Projects** break down into epics, features, and tasks

This isn't arbitrary—hierarchical thinking is how we manage complexity. It lets us zoom out to see the big picture or zoom in to focus on specifics—and switch between these views effortlessly. It provides context: knowing something is "under" a topic tells you what it relates to.

What makes knowledge truly interesting is where hierarchies intersect. "Color Theory" belongs under both Art and Physics. "Game Theory" connects Mathematics and Economics. "Nutrition" bridges Chemistry and Health. These intersections are where insight lives.

Good knowledge systems should support this natural way of thinking.

## The Problem

Traditional ways of organizing information don't match how knowledge actually works.

### Directories: One Place Only

File systems force everything into a single hierarchy. A note about "Meditation" must live in either `/health/` or `/productivity/`—it can't naturally exist in both.

This leads to:

- Arbitrary placement decisions
- Broken references when folders change
- Duplicates or "misc" folders when nothing fits

But knowledge isn't a single tree—it's interconnected.

### Tags: Flexible but Shallow

Tags allow multiple categories, but they lack structure:

- No order or priority
- No explanation of relationships
- No hierarchy
- No grouping within a category

A note tagged `#health #productivity #mindfulness` doesn't explain how these topics relate or which one is primary.

## The Solution: Inclusion Links

An **inclusion link** is a markdown link placed on its own line:

``` markdown
# Photography

[Composition](composition.md)

[Lighting](lighting.md)

[Post-Processing](post-processing.md)
```

When a link appears on its own line, it defines structure: "Photography" becomes the parent of the linked documents.

This simple rule turns plain markdown into a structured, navigable system.

## What This Enables

### Multiple Contexts (Polyhierarchy)

A document can belong to multiple parents.

**In `health-practices.md`:**

``` markdown
# Health Practices

[Meditation](meditation.md)
```

**In `productivity-tools.md`:**

``` markdown
# Productivity Tools

[Meditation](meditation.md)
```

The same document appears in both contexts without duplication.

### Structure with Meaning

Unlike tags, inclusion links allow ordering, grouping, and explanation:

``` markdown
# Psychology

## Cognitive

How we encode, store, and retrieve information

[Memory](memory.md)

Biases, heuristics, and rational choice

[Decision Making](decision-making.md)

## Behavioral

Cue, routine, reward loops

[Habit Formation](habit-formation.md)

```

You're not just grouping items—you're adding context.

### Navigable Hierarchies

By linking documents together, you naturally create paths through your knowledge:

```
Knowledge Base > Psychology > Cognitive > Memory
Knowledge Base > Photography > Composition
```

This makes navigation and search more meaningful—you see not just what matches, but where it fits.

### Context Flows Downward

When a document appears under a parent, it inherits meaning from that relationship. A "Memory" note under "Psychology" is understood differently than "Memory" under "Computer Architecture."

The parent page can also add explicit context:

``` markdown
# Cognitive Psychology

Foundation of learning and recall

[Memory](memory.md)

Selective focus and its limits

[Attention](attention.md)
```

These descriptions aren't part of the child documents—they live in the parent, explaining why each child belongs here.

### Flexible Level of Detail

You can control how much information is visible. With `--depth 1`, you see only immediate children:

```
# Psychology
- Memory
- Decision Making
- Habit Formation
```

With `--depth 3`, the full subtree expands:

```
# Psychology
- Memory
  - Working Memory
    - Capacity Limits
    - Chunking Strategies
  - Long-term Memory
- Decision Making
- Habit Formation
```

Use [Extract Actions](feature-extract.md) to move details into separate documents, or [Inline Notes](feature-inline.md) to expand linked content in place.

## Inclusion Links vs Inline Links

### Inclusion Links (Structure)

Inclusion links define parent-child relationships and are used for:

- Navigation
- Hierarchical traversal (`--depth`)
- Structured views

### Inline Links (References)

Links inside text create conceptual connections:

``` markdown
# Habit Formation

The [Habit Loop](habit-loop.md) consists of cue, routine, and reward.
This process is driven by [Dopamine Pathways](dopamine.md) in the brain.
```

These links:

- Create backlinks
- Show relationships between ideas
- Do not affect structure

### Why This Distinction Matters

When retrieving content with depth:

``` bash
iwe retrieve psychology --depth 2
```

The `--depth` flag controls how many levels of inclusion links to expand. See [IWE Retrieve](cli-retrieve.md) for details.

- Inclusion links expand into full content
- Inline links remain references only

This keeps structure clean while preserving connections between ideas.

## Summary

Inclusion links give you structure without rigidity:

- Documents can exist in multiple contexts
- Ordering, grouping, and meaning are explicit
- Navigation becomes contextual

For a deeper understanding of how IWE represents these relationships internally, see [Data Model](data-model.md).
