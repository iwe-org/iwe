# Maps of Content

Personal Knowledge Management (PKM) systems revolve around managing a graph of notes. However, every Markdown file is a graph in itself. Let me explain with an example:

``` markdown
# Header 1 

Paragraph 1
```

Here, `Header 1` is the logical parent of `Paragraph 1`.

``` markdown
# Header 1 

## Header 2

Paragraph 1
```

In this example, `Paragraph 1` belongs to `Header 2`, which in turn belongs to `Header 1`.

You get the idea: it effectively forms a tree (which is also a graph) of text blocks.

So, why does this matter? Suppose I want to find something in my notes graph. I can achieve better results using context-aware search. For example:

``` markdown
# Projects

## My new shiny thing

Paragraph 1
```

If I type "Proj" in the search bar, I should get two matches:

```
Projects > My new shiny thing 
Projects
```

And if I type "shiny," the search result should be:

```
Projects > My new shiny thing 
```

This way, I gain a bit of context.

Okay, it sounds promising, but how can I scale this to thousands of notes and multiple contexts?

It's simple. Just use the "Maps of Content" (MOC) approach:

``` markdown
# Projects

[[My new shiny thing]]

[[The old thing]]

[[The old thing 2]]
```

This will yield similar search results:

```
Projects > My new shiny thing 
Projects > The old thing 2
Projects > The old thing
Projects
```

With this approach, you can delve as deep as you like:

``` markdown
# Personal

[[Projects]]
```

```
Personal > Projects > My new shiny thing 
Personal > Projects > The old thing 2
Personal > Projects > The old thing
Personal > Projects
```

## Structure Notes: Beyond Simple Lists

While Maps of Content provide hierarchical organization, you can create even more sophisticated structures using **Structure Notes** - meta-notes that explicitly document relationships between other notes.

### Hub Notes as Entry Points

Not every relevant note needs to be listed directly in your main MOCs. Instead, create central hub notes that serve as entry points to specific topics. These hub notes contain links to the most important notes on a subject, which then connect to related materials.

For example, instead of listing every single project-related note in your main Projects MOC, you might have:

``` markdown
# Projects

[[Active Projects]] - Current work and ongoing initiatives

[[Project Templates]] - Standardized approaches and methodologies  

[[Project Archive]] - Completed projects and lessons learned
```

Each hub note then contains its own detailed connections and references.

### Types of Structural Relationships

Structure Notes can capture different types of relationships beyond simple hierarchies:

#### Sequential Structures

Some knowledge follows logical sequences or chains of reasoning. A Structure Note can map these step-by-step progressions:

``` markdown
# Feature Development Process

[[Requirements Gathering]] → [[Design Phase]] → [[Implementation]] → [[Testing]] → [[Deployment]]

Each phase builds on the previous, with dependencies clearly mapped.
```

#### Overlapping Connections

Unlike strict trees, knowledge often has cross-connections. A note about "API Design Patterns" might belong both in your "Software Architecture" MOC and your "Web Development" MOC, creating a network structure rather than a simple hierarchy.

#### Thematic Clustering

Group related concepts that share common themes or applications:

``` markdown
# Mental Models for Problem Solving

## Analysis Models

[[Root Cause Analysis]]

[[Five Whys Technique]]

[[Fishbone Diagrams]]

## Systems Models  

[[Feedback Loops]]

[[Leverage Points]]

[[Network Effects]]
```

### Creating Effective Structure Notes

When building Structure Notes:

1.  **Focus on Relationships**: Don't just list notes - explain how they connect and why they belong together
2.  **Use Multiple Structures**: Combine hierarchical, sequential, and network structures as appropriate
3.  **Maintain Entry Points**: Ensure your most important Structure Notes are linked from main MOCs
4.  **Update Regularly**: As you create new notes, update relevant Structure Notes to include them

This approach transforms your knowledge base from a simple collection of linked notes into a sophisticated web of interconnected ideas, making it easier to navigate, discover connections, and generate new insights.

## Learn more

- A great explanation of what the structure notes are and how to use them is available [here](https://zettelkasten.de/introduction/#structure-notes)
- MOC's overview is available [here](https://notes.linkingyourthinking.com/Cards/MOCs+Overview)
