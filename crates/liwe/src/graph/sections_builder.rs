use crate::{
    graph::builder::GraphBuilder,
    model::{Key, LineRange, NodesMap},
};
use itertools::Itertools;

use crate::model::document::DocumentBlock::{
    self, BlockQuote, BulletList, CodeBlock, Div, Header, HorizontalRule, OrderedList, Para, Plain,
    RawBlock,
};
use crate::model::document::DocumentBlocks;
use crate::model::graph::to_graph_inlines;

type Range = std::ops::Range<usize>;

pub struct SectionsBuilder<'a> {
    builder: &'a mut GraphBuilder<'a>,
    nodes_map: NodesMap,
    key: Key,
}

impl<'a> SectionsBuilder<'a> {
    pub fn nodes_map(&self) -> NodesMap {
        self.nodes_map.clone()
    }

    pub fn new(
        builder: &'a mut GraphBuilder<'a>,
        content: &DocumentBlocks,
        key: &Key,
    ) -> SectionsBuilder<'a> {
        let mut builder = SectionsBuilder {
            builder,
            nodes_map: vec![],
            key: key.clone(),
        };
        builder.process_blocks(0..content.len(), content);
        builder
    }

    pub fn process_blocks(&mut self, range: Range, content: &DocumentBlocks) {
        // 1. append all non-header blocks until first header
        // 2. take the rest and split into sections
        // 3. call process_sections for each section
        if range.is_empty() {
            return;
        }

        self.builder.set_insert(true);
        let first_header = first_header(range.clone(), content);
        let pre_header_range = range.start..first_header.unwrap_or(range.end);
        for i in pre_header_range.clone() {
            self.block(&content[i]);
        }

        if first_header_level(range.clone(), content).is_none() {
            return;
        }

        let positions = content
            .iter()
            .positions(|x| match x {
                Header(header) => {
                    header.level <= first_header_level(range.clone(), content).unwrap()
                }
                _ => false,
            })
            .filter(|&x| x >= first_header.unwrap_or(range.start))
            .filter(|&x| x <= range.end)
            .collect_vec();

        let ranges = ranges(positions, range.end);

        for i in ranges {
            self.process_section(i, &content);
        }
    }

    pub fn process_section(&mut self, range: Range, blocks: &DocumentBlocks) {
        // section always starts with a header
        // 1. append the header
        // 2. call process_blocks for the rest

        if range.is_empty() {
            return;
        }

        self.section_block(&blocks[range.start]);

        let id = self.builder.id();
        self.process_blocks(range.start + 1..range.end, blocks);
        self.builder.set_id(id)
    }

    pub fn section_block(&mut self, block: &DocumentBlock) {
        match block.clone() {
            Para(para) => {
                self.builder
                    .section(to_graph_inlines(&para.inlines, &self.key.parent()));
                self.set_lines_range(para.line_range);
            }
            Plain(plain) => {
                self.builder
                    .section(to_graph_inlines(&plain.inlines, &self.key.parent()));
                self.set_lines_range(plain.line_range);
            }
            Header(header) => {
                self.builder
                    .section(to_graph_inlines(&header.inlines, &self.key.parent()));
                self.set_lines_range(header.line_range);
            }
            Div(div) => {
                self.section_block(div.blocks.first().unwrap());
            }
            _ => {
                panic!("section block panic for: {:?}", block)
            }
        };
    }

    pub fn block(&mut self, block: &DocumentBlock) {
        match block.clone() {
            CodeBlock(code_block) => {
                self.builder.raw(&code_block.text, code_block.lang);
                self.set_lines_range(code_block.line_range);
            }
            RawBlock(raw_block) => {
                self.builder.raw(&raw_block.text, Some(raw_block.format));
                self.set_lines_range(raw_block.line_range);
            }
            Plain(plain) => {
                self.builder
                    .leaf(to_graph_inlines(&plain.inlines, &self.key.parent()));
                self.set_lines_range(plain.line_range);
            }
            Para(para) => {
                if block.is_ref() {
                    self.builder.reference_with_text(
                        &Key::from_rel_link_url(&block.url().unwrap(), &self.key.parent()),
                        &block.ref_text().unwrap(),
                        block.ref_type().unwrap(),
                    )
                } else {
                    self.builder
                        .leaf(to_graph_inlines(&para.inlines, &self.key.parent()))
                }
                self.set_lines_range(para.line_range);
            }
            BulletList(list) => {
                self.builder.bullet_list();
                self.builder.set_insert(true);
                let id = self.builder.id();

                for b in list.items.iter() {
                    self.process_section(0..b.len(), b);
                }

                self.builder.set_id(id);
            }
            OrderedList(list) => {
                self.builder.ordered_list();
                self.builder.set_insert(true);
                let id = self.builder.id();

                for b in list.items.iter() {
                    self.process_section(0..b.len(), b);
                }

                self.builder.set_id(id);
            }
            BlockQuote(quote) => {
                self.builder.quote();
                self.set_lines_range(quote.line_range);
                let id = self.builder.id();
                SectionsBuilder::new(
                    &mut self.builder.graph().builder(id),
                    &quote.blocks,
                    &self.key,
                );
            }
            HorizontalRule(rule) => {
                self.builder.horizontal_rule();
                self.set_lines_range(rule.line_range);
            }
            Div(div) => {
                self.section_block(div.blocks.first().unwrap());
            }
            Header(_) => {
                panic!("Unexpected block type, headers should be process outside of this block")
            }
            DocumentBlock::Table(table) => {
                let header = table
                    .header
                    .iter()
                    .map(|cell| to_graph_inlines(&cell, &self.key.parent()))
                    .map(|inlines| self.builder.graph().add_line(inlines))
                    .collect_vec();

                let rows = table
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|cell| to_graph_inlines(&cell, &self.key.parent()))
                            .map(|inlines| self.builder.graph().add_line(inlines))
                            .collect_vec()
                    })
                    .collect_vec();

                self.builder.table(header, table.alignment, rows);
                self.set_lines_range(table.line_range);
            }
        };
    }

    fn set_lines_range(&mut self, line_range: LineRange) {
        self.nodes_map.push((self.builder.node().id(), line_range));
    }
}

fn ranges(positions: Vec<usize>, end: usize) -> Vec<Range> {
    let mut ranges: Vec<Range> = vec![];

    if positions.is_empty() {
        return vec![];
    }

    for i in 0..positions.len() - 1 {
        ranges.push(positions[i]..positions[i + 1]);
    }
    if positions[positions.len() - 1] < end {
        ranges.push(positions[positions.len() - 1]..end);
    }
    ranges
}

pub fn first_header_level(range: Range, content: &DocumentBlocks) -> Option<u8> {
    range.into_iter().find_map(|i| match content[i].clone() {
        Header(header) => Some(header.level),
        _ => None,
    })
}

pub fn first_header(range: Range, content: &DocumentBlocks) -> Option<usize> {
    range.into_iter().find(|i| match content[*i].clone() {
        Header(_) => true,
        _ => false,
    })
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use crate::{graph::Graph, markdown::MarkdownReader, model::LineRange};

    use crate::model::{Key, NodeId};

    #[test]
    pub fn code_block_no_lang() {
        assert_eq(
            Graph::with(|graph| {
                graph.build_key(&"key".into()).raw("code\n", None);
            }),
            indoc! {"
            ```
            code
            ```
            "},
        )
    }

    #[test]
    pub fn code_block_with_lang() {
        assert_eq(
            Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .raw("code\n", Some("lang".to_string()));
            }),
            indoc! {"
            ``` lang
            code
            ```
            "},
        )
    }

    #[test]
    pub fn header2() {
        assert_eq(
            Graph::with(|graph| {
                graph.build_key(&"key".into()).section_text("header");
            }),
            indoc! {"
            # header
            "},
        )
    }

    #[test]
    pub fn sub_header_section() {
        assert_eq(
            Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .section_text_and("header", |s| {
                        s.section_text("sub-header");
                    });
            }),
            indoc! {"
            # header

            ## sub-header
            "},
        )
    }

    #[test]
    pub fn sub_header_and_header_after() {
        assert_eq(
            Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .section_text_and("header", |s| {
                        s.section_text("sub-header");
                    })
                    .section_text("header-2");
            }),
            indoc! {"
            # header

            ## sub-header

            # header-2
            "},
        )
    }

    #[test]
    pub fn headers_level_normalization() {
        assert_eq(
            Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .section_text_and("header", |s| {
                        s.section_text("sub-header");
                    })
                    .section_text("header-2");
            }),
            indoc! {"
            # header

            ### sub-header

            # header-2
            "},
        )
    }

    #[test]
    pub fn headers_at_top_level() {
        assert_eq(
            Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .section_text_and("header", |s| {
                        s.leaf_text("item");
                    })
                    .section_text("header-2");
            }),
            indoc! {"
            # header

            item

            # header-2
            "},
        )
    }

    #[test]
    pub fn sub_header_before_top_level_header() {
        assert_eq(
            Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .section_text("header")
                    .section_text("header-2");
            }),
            indoc! {"
            ## header

            # header-2
            "},
        )
    }

    #[test]
    pub fn list_item_item() {
        assert_eq(
            Graph::with(|graph| {
                graph.build_key(&"key".into()).bullet_list_and(|l| {
                    l.section_text("item-1");
                });
            }),
            indoc! {"
            - item-1
            "},
        )
    }

    #[test]
    pub fn two_items_list() {
        assert_eq(
            Graph::with(|graph| {
                graph.build_key(&"key".into()).bullet_list_and(|l| {
                    l.section_text("item-1").section_text("item-2");
                });
            }),
            indoc! {"
            - item-1
            - item-2
            "},
        )
    }

    #[test]
    pub fn header_position() {
        assert_position_eq(
            indoc! {"
            # header
            "},
            1,
            0..1,
        )
    }

    #[test]
    pub fn para_position() {
        assert_position_eq(
            indoc! {"
            para
            "},
            1,
            0..1,
        )
    }

    #[test]
    pub fn header_2_position() {
        assert_position_eq(
            indoc! {"
            # header

            # header
            "},
            2,
            2..3,
        )
    }

    #[test]
    pub fn para_in_header_position() {
        assert_position_eq(
            indoc! {"
            # header

            para
            "},
            2,
            2..3,
        )
    }

    #[test]
    pub fn multiline_para() {
        assert_position_eq(
            indoc! {"
            para
            para 2
            "},
            1,
            0..2,
        )
    }

    #[test]
    pub fn multiline_code() {
        assert_position_eq(
            indoc! {"
            ``` lang
            code
            code 2
            ```
            "},
            1,
            0..3,
        )
    }

    #[test]
    pub fn multiline_list() {
        assert_position_eq(
            indoc! {"
            - item
            - item-2
            "},
            2,
            0..1,
        );
        assert_position_eq(
            indoc! {"
            - item
            - item-2
            "},
            3,
            1..2,
        );
    }

    #[test]
    pub fn multiline_list_nested() {
        assert_position_eq(
            indoc! {"
            - item
              - item-2
            "},
            4,
            1..2,
        );
    }

    fn assert_eq(expected: Graph, actual: &str) {
        let mut actual_graph = Graph::new();
        actual_graph.from_markdown(Key::from_file_name("key"), actual, MarkdownReader::new());

        assert_eq!(expected, actual_graph);
    }

    fn assert_position_eq(actual: &str, node_id: NodeId, range: LineRange) {
        let mut actual_graph = Graph::new();
        actual_graph.from_markdown(Key::from_file_name("key"), actual, MarkdownReader::new());

        assert_eq!(
            range,
            actual_graph
                .node_line_range(node_id)
                .expect("to have a range")
        );
    }

    // List structure

    // - item
    // - item
    // [ [Palin], [Plain] ]

    // - item
    //   - item
    //   - item
    // [ [Plain, BulletList( [ [Plain], [Plain] ] ) ] ]

    // - item
    //   - item
    // [ [Plain, BulletList( [ [Plain] ] ) ] ]

    // - item
    //   - item
    //     - item
    // [ [Plain, BulletList( [[Plain, BulletList( [[Plain]] ) ] ] ) ] ]

    // - item 1
    //   - sub item 1
    // - item 2
    //   - sub item 2
    // [ [Plain, BulletList([[Plain]])], [Plain, BulletList([[Plain]])]]
}
