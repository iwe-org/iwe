# Basic journal example

Lets take this Markdown journal as an example.

`📄 journal-2025.md`

``` markdown
# Journal, 2025

## Week 3 - Coffee week

This week, I tried three types of coffee: the **cappuccino** with its bold espresso and frothy milk offering a delightful texture, the **latte** which envelops espresso and milk in a comforting embrace perfect for leisurely mornings, and the **cortado**, a balanced blend of espresso and milk that brings peace to the taste buds.

### Jan 26, 2025 - Cappuccino

It's cappuccino day. The classic Italian masterpiece, where espresso meets a frothy cloud of milk, creating a delightful contrast of bold and creamy. It's like sipping on a caffeine-infused cloud, perfect for anyone wanting to add a little texture to their daily routine.
### Jan 25, 2025 - Latte

As warm as a hug from an old friend, the latte wraps espresso and milk in a snug embrace. With a canvas for barista art, it’s not just a drink, but a little piece of serenity in a cup for those more leisurely mornings when taking it slow is the only option.

### Jan 24, 2025 - Cortado

I had an amazing cortado today. It's when espresso and milk meet halfway in a charming truce, the cortado emerges. It's the perfect compromise, bringing balance to your coffee routine and peace to your taste buds.
```

This kind of a document can grow very fast. IWE can transform it by *collapsing* sections into *block-references*. This transformation maintains the document hierarchy while reducing level of details.

`📄 journal-2025.md`

``` markdown
# Journal, 2025

## Week 3 - Coffee week

This week, I tried three types of coffee: the **cappuccino** with its bold espresso and frothy milk offering a delightful texture, the **latte** which envelops espresso and milk in a comforting embrace perfect for leisurely mornings, and the **cortado**, a balanced blend of espresso and milk that brings peace to the taste buds.

[Jan 26, 2025 - Cappuccino](jan-26)

[Jan 25, 2025 - Latte](jan-25)

[Jan 24, 2025 - Cortado](jan-24)
```

And three daily files:

`📄 jan-26.md`

`📄 jan-25.md`

`📄 jan-24.md`

You can repeat this again, adding as many levels as necessary

`📄 journal-2025.md`

``` markdown
# Journal, 2025

[Week 3 - Coffee week](2025-W3)
```

`📄 2025-W3.md`

`📄 jan-26.md`

`📄 jan-25.md`

`📄 jan-24.md`

As a result of this decomposition, each document is much simpler while the original hierarchy is preserved. It's also a perfectly valid markdown with no additional syntax.

IWE support automated actions for graph transformations like this and it can just as easily reconstruct the **original** document buy combining the extracted content together preserving correct headings structure.
