use super::*;
use iced::widget::{button, column, container, pick_list, row, rule, scrollable, stack, text, text_input, Space};

impl State {
    pub(crate) fn view<'a>(
        &'a self,
        settings: &'a Settings,
        anim: &'a AnimState,
    ) -> Element<'a, Message> {
        let body = match self.session.as_ref() {
            Some(session) => session.view(settings, anim, self.shipping(), self.readout, self.dials),
            None => self.vacant_view(settings, anim),
        };

        stack![body, self.view_notice()].into()
    }

    fn vacant_view<'a>(
        &'a self,
        settings: &'a Settings,
        anim: &'a AnimState,
    ) -> Element<'a, Message> {
        let (side, stage): (Element<'_, Message>, Element<'_, Message>) = match self.mode {
            Mode::Atlas => (
                column![vacant_cuts(), vacant_slices()].spacing(GAP).height(Length::Fill).into(),
                stack![console_card(centred(NO_SET_HINT)), console_edge()].into(),
            ),
            Mode::Entity => {
                let shown = match self.readout {
                    Readout::Facts => facts_table(Focus::Curve, None, None, None),
                    Readout::Timeline => timeline::vacant(NO_SET_NOTICE),
                };

                (
                    column![console_card(centred(NO_SET_NOTICE)), vacant_keys()]
                        .spacing(GAP)
                        .height(Length::Fill)
                        .into(),
                    column![
                        container(self.idle.view(settings, anim).map(Message::Viewer))
                            .width(Length::Fill)
                            .height(Length::Fill),
                        strip(shown, &settings.studio, false, self.readout, self.dials)
                    ]
                    .spacing(GAP)
                    .into(),
                )
            }
        };

        let body = row![panel_frame(self.mode, false, self.shipping(), side), stage].spacing(GAP);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(BODY_PADDING)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                ..container::Style::default()
            })
            .into()
    }

    fn view_notice(&self) -> Element<'_, Message> {
        let mut said = column![theme::centered_text(self.notice_text.as_str()).size(NOTICE_TEXT_SIZE)]
            .align_x(Horizontal::Center);

        if let Some(hint) = self.notice_hint {
            said = said.push(
                theme::centered_text(hint)
                    .size(LABEL_SIZE)
                    .style(|theme: &Theme| text::Style { color: Some(theme::weak_text_color(theme)) }),
            );
        }

        let banner = container(said)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(Padding {
                top: NOTICE_OVERHANG + NOTICE_PAD_Y,
                right: NOTICE_PAD_X,
                bottom: NOTICE_PAD_Y,
                left: NOTICE_PAD_X,
            })
            .style(theme::notice_banner);

        container(slide(banner, self.raised, Slide::Up).floating())
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Top)
            .padding(Padding::default().top(-NOTICE_OVERHANG))
            .into()
    }
}

impl Session {
    fn ghost(&self, cargo: Cargo, at: Point) -> Element<'_, Message> {
        let label = self
            .rows
            .iter()
            .find(|row| row.cargo() == Some(cargo))
            .map_or("", |row| row.label.as_str());

        let carried = text(label)
            .font(glyphs::mono())
            .size(TREE_TEXT_SIZE)
            .wrapping(text::Wrapping::None)
            .style(|theme: &Theme| text::Style {
                color: Some(Color { a: GHOST_INK, ..theme.palette().text }),
            });

        let card = container(carried).padding([GHOST_PAD, GHOST_PAD * 2.0]).style(|theme: &Theme| {
            let palette = theme.palette();

            container::Style {
                background: Some(Color { a: GHOST_FILL, ..palette.primary }.into()),
                border: Border::default()
                    .rounded(4.0)
                    .width(1.0)
                    .color(Color { a: GHOST_EDGE, ..palette.primary }),
                ..container::Style::default()
            }
        });

        let width = CHAR_WIDTH * glyphs::columns(label) + GHOST_PAD * 4.0;
        let height = TREE_TEXT_SIZE + GHOST_PAD * 2.0;

        let placed = Padding::default()
            .top((at.y - height / 2.0).max(0.0))
            .left((at.x - width / 2.0).max(0.0));

        container(card).padding(placed).width(Length::Fill).height(Length::Fill).into()
    }

    fn view<'a>(
        &'a self,
        settings: &'a Settings,
        anim: &'a AnimState,
        shipping: Shipping,
        readout: Readout,
        dials: usize,
    ) -> Element<'a, Message> {
        let handled = self.viewer.resolved() && !self.viewer.selecting();

        let showing: Element<'_, Message> = if handled {
            editor::deflect(
                stack![
                    self.viewer.stage_view(settings).map(Message::Viewer),
                    self.gizmo.view(
                        self.chosen_part(),
                        self.placed.clone(),
                        self.viewer.camera(),
                    ),
                    self.viewer.controls_view(anim, &self.alarms).map(Message::Viewer),
                ],
                true,
            )
        } else if self.viewer.resolved() {
            self.viewer.view(settings, anim).map(Message::Viewer)
        } else {
            container(text(LOADING_NOTICE).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        let stage = container(showing).width(Length::Fill).height(Length::Fill);

        let right: Element<'_, Message> = match self.mode {
            Mode::Atlas => self.canvas(),
            _ => column![stage, self.strip(settings, readout, dials)].spacing(GAP).into(),
        };

        let body = row![self.side(shipping), right].spacing(GAP);

        let content = container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(BODY_PADDING)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                ..container::Style::default()
            });

        let mut layers = stack![content];

        if self.drag != Drag::Idle {
            let carried = self.drag.carrying();
            let ghost: Element<'_, Message> = match self.drag {
                Drag::Moving { cargo, at, .. } => self.ghost(cargo, at),
                _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
            };

            layers = layers.push(
                mouse_area(ghost)
                    .interaction(match carried.is_some() {
                        true => mouse::Interaction::Grabbing,
                        false => mouse::Interaction::Grab,
                    })
                    .on_move(Message::DragMove)
                    .on_release(Message::DragEnd),
            );
        }

        layers.into()
    }

    fn side(&self, shipping: Shipping) -> Element<'_, Message> {
        let channelled =
            self.focus == Focus::Curve && self.draft.as_ref().is_some_and(|draft| draft.track.is_some());

        let body = match (self.mode, channelled) {
            (Mode::Atlas, _) => column![self.loaders(), self.cuts()],
            (Mode::Entity, true) => column![self.tree(), self.keys()],
            (Mode::Entity, false) => column![self.tree(), self.fields()],
        };

        panel_frame(self.mode, true, shipping, body.spacing(GAP).height(Length::Fill).into())
    }

    fn canvas(&self) -> Element<'_, Message> {
        let showing: Element<'_, Message> = match self.atlas.as_ref() {
            Some(atlas) => self
                .picture
                .view_outlined(&atlas.source, &atlas.outlines, self.framing.is_some())
                .map(Message::Picture),
            None => centred(NO_ATLAS_NOTICE),
        };

        let framing = self.framing.is_some();

        stack![
            editor::suppress(console_card(showing), framing),
            overlay::hint(FRAME_HINT, framing),
            console_edge()
        ]
        .into()
    }

    fn loaders(&self) -> Element<'_, Message> {
        let sheet = self.viewer.selected_sheet().and_then(named).unwrap_or_default();
        let cuts = self.viewer.selected_cuts().and_then(named).unwrap_or_default();
        let atlas = self.atlas.as_ref();

        let size = self
            .viewer
            .rig()
            .and_then(|rig| rig.sheet.image_data.as_ref())
            .map_or_else(String::new, |art| format!("{} \u{00d7} {}", art.width(), art.height()));

        let facts = [
            ("Sheet", sheet),
            ("Cuts", cuts),
            ("Size", size),
            ("Regions", atlas.map_or_else(String::new, |atlas| atlas.doc.count().to_string())),
        ];

        facts_card(facts)
    }

    fn cuts(&self) -> Element<'_, Message> {
        let table = container(
            container(responsive(move |size: Size| self.view_cuts(size)))
                .padding(theme::CONSOLE_BORDER_WIDTH),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container);

        let notice: Element<'_, Message> =
            match self.atlas.as_ref().is_some_and(|atlas| atlas.backing.failed) {
                true => text(WRITE_FAILED_NOTICE).size(LABEL_SIZE).style(text::danger).into(),
                false => Space::new().height(Length::Fixed(0.0)).into(),
            };

        let add = footer_button("Add Cut", self.atlas.as_ref().map(|_| Message::AddCut));

        column![notice, table, add].spacing(ROW_GAP).height(Length::Fill).into()
    }

    fn view_cuts(&self, size: Size) -> Element<'_, Message> {
        let atlas = self.atlas.as_ref();
        let rows = atlas.map_or(0, |atlas| atlas.inputs.len());

        if rows == 0 {
            let blank = container(theme::centered_text(NO_CUTS_NOTICE).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            return column![cuts_header(0.0), blank].width(Length::Fill).height(Length::Fill).into();
        }

        let body = (size.height - KEY_HEAD_HEIGHT).max(0.0);
        let tail = if rows as f32 * KEY_ROW_HEIGHT > body { SCROLLBAR_ALLOWANCE } else { 0.0 };
        let width = (size.width - tail).max(0.0);

        let RowWindow { range, pad_before, pad_after } =
            row_window::compute_with(rows, body, self.strip_scroll, KEY_ROW_HEIGHT, 0.0);

        let mut list = Column::with_capacity(range.len() + 2).width(Length::Fixed(width));

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for at in range {
            if let Some(atlas) = atlas {
                let framing = self.framing == Some(at);

                let armed = self.slicing.armed_for(&at);

                list = list.push(atlas.cut_row(at, framing, armed, self.slice == Some(at), width));
            }
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        let scrolled = smooth_scroll(
            scrollable(list)
                .id(self.strip_id.clone())
                .direction(scrollable::Direction::Vertical(bar()))
                .on_scroll(|viewport| Message::StripScrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        );

        column![cuts_header(tail), scrolled].width(Length::Fill).height(Length::Fill).into()
    }

    fn tree(&self) -> Element<'_, Message> {
        let by_part = self.focus == Focus::Part;
        let picked = match by_part {
            true => self.pose.as_ref().and_then(|pose| pose.part),
            false => self.draft.as_ref().and_then(|draft| draft.track),
        };

        let dragged = self.drag.carrying();
        let landing = self.drag.landing();
        let body = responsive(move |size: Size| self.view_rows(size, picked, by_part, dragged, landing));

        container(container(body).padding(theme::CONSOLE_BORDER_WIDTH))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::mock_console_container)
            .into()
    }

    fn view_rows(
        &self,
        size: Size,
        picked: Option<usize>,
        by_part: bool,
        dragged: Option<Cargo>,
        landing: Option<Landing>,
    ) -> Element<'_, Message> {
        let tail = if self.widest > size.width { SCROLL_TAIL } else { 0.0 };

        let RowWindow { range, pad_before, pad_after } = row_window::compute_with(
            self.rows.len(),
            size.height - tail,
            self.scroll,
            ROW_HEIGHT,
            ROW_SPACING,
        );

        let width = self.widest.max(size.width - SCROLLBAR_ALLOWANCE);
        let mut list = Column::with_capacity(range.len() + 3).spacing(ROW_SPACING);

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for index in range {
            let Some(row) = self.rows.get(index) else {
                continue;
            };

            let carried = dragged.is_some_and(|cargo| row.cargo() == Some(cargo));
            let onto = landing.and_then(|landing| landing.mark(index, self.rows.len() - 1));

            list = list.push(row.view(index, picked, by_part, carried, onto, width));
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        if tail > 0.0 {
            list = list.push(space().height(Length::Fixed(tail)));
        }

        smooth_scroll(
            scrollable(list)
                .id(self.scroll_id.clone())
                .direction(both_ways())
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y, viewport.bounds().height))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .into()
    }

    fn strip<'a>(&'a self, settings: &'a Settings, readout: Readout, dials: usize) -> Element<'a, Message> {
        let index = self.chosen().and_then(|at| i32::try_from(at).ok());

        let part = index
            .and_then(|index| usize::try_from(index).ok())
            .zip(self.viewer.rig())
            .and_then(|(at, rig)| rig.model.parts.get(at));

        let shown = match readout {
            Readout::Facts => facts_table(self.focus, index, part, self.viewer.rig()),
            Readout::Timeline => self.board(),
        };

        strip(shown, &settings.studio, self.animated(), readout, dials)
    }

    fn board(&self) -> Element<'_, Message> {
        let chosen = self.chosen();
        let doc = self.draft.as_ref().map(|draft| &draft.doc);
        let picked = self.draft.as_ref().and_then(|draft| draft.track);

        let (lanes, cadence) = match doc.zip(chosen) {
            Some((doc, part)) => (
                timeline::lanes(doc, part),
                i32::try_from(part).map_or_else(|_| Cadence::of(&[]), |at| doc.cadence(at)),
            ),
            None => (Vec::new(), Cadence::of(&[])),
        };

        self.timeline.view(chosen, lanes, cadence, self.viewer.frame(), picked)
    }

    fn keys(&self) -> Element<'_, Message> {
        let draft = self.draft.as_ref();

        let add = footer_button("Add Keyframe", draft.map(|_| Message::AddKey));

        let active = draft
            .and_then(|draft| draft.curve())
            .and_then(|track| curve::playhead(track, self.viewer.frame()))
            .map(|playhead| playhead.key);

        let notice: Element<'_, Message> = match draft.is_some_and(|draft| draft.backing.failed) {
            true => text(WRITE_FAILED_NOTICE).size(LABEL_SIZE).style(text::danger).into(),
            false => Space::new().height(Length::Fixed(0.0)).into(),
        };

        let table = container(
            container(responsive(move |size: Size| self.view_keys(size, active)))
                .padding(theme::CONSOLE_BORDER_WIDTH),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container);

        let footer: Element<'_, Message> = match draft {
            Some(draft) => row![draft.looping(), add].spacing(GAP).align_y(Vertical::Center).into(),
            None => add,
        };

        column![notice, table, footer].spacing(ROW_GAP).height(Length::Fill).into()
    }

    fn view_keys(&self, size: Size, active: Option<usize>) -> Element<'_, Message> {
        let draft = self.draft.as_ref();
        let rows = draft.map_or(0, |draft| draft.inputs.len());
        let body = (size.height - KEY_HEAD_HEIGHT).max(0.0);
        let spans = rows as f32 * KEY_ROW_HEIGHT;
        let tail = if spans > body { SCROLLBAR_ALLOWANCE } else { 0.0 };

        let RowWindow { range, pad_before, pad_after } =
            row_window::compute_with(rows, body, self.strip_scroll, KEY_ROW_HEIGHT, 0.0);

        let width = (size.width - tail).max(0.0);
        let mut list = Column::with_capacity(range.len() + 2).width(Length::Fixed(width));

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for at in range {
            if let Some(draft) = draft {
                let armed = self.confirm.armed_for(&at);

                list = list.push(draft.key_row(at, spanned(at, active, rows), at, armed, width));
            }
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        if rows == 0 {
            let blank = container(theme::centered_text(self.vacancy()).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            return column![keys_header(0.0), blank].width(Length::Fill).height(Length::Fill).into();
        }

        let scrolled = smooth_scroll(
            scrollable(list)
                .id(self.strip_id.clone())
                .direction(scrollable::Direction::Vertical(bar()))
                .on_scroll(|viewport| Message::StripScrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        );

        column![keys_header(tail), scrolled].width(Length::Fill).height(Length::Fill).into()
    }

    fn fields(&self) -> Element<'_, Message> {
        let pose = self.pose.as_ref();

        let add = footer_button("Add Part", pose.map(|_| Message::AddPart));

        let notice: Element<'_, Message> = match pose.is_some_and(|pose| pose.backing.failed) {
            true => text(WRITE_FAILED_NOTICE).size(LABEL_SIZE).style(text::danger).into(),
            false => Space::new().height(Length::Fixed(0.0)).into(),
        };

        let table = container(
            container(responsive(move |size: Size| self.view_fields(size)))
                .padding(theme::CONSOLE_BORDER_WIDTH),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container);

        let footer: Element<'_, Message> = match pose {
            Some(pose) => row![pose.aligning(), add].spacing(GAP).align_y(Vertical::Center).into(),
            None => add,
        };

        column![notice, table, footer].spacing(ROW_GAP).height(Length::Fill).into()
    }

    fn view_fields(&self, size: Size) -> Element<'_, Message> {
        let pose = self.pose.as_ref();
        let rows = pose.map_or(0, |pose| pose.inputs.len());

        if rows == 0 {
            let blank = container(theme::centered_text(self.absence()).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            let bare: Element<'_, Message> =
                column![fields_header(0.0), blank].width(Length::Fill).height(Length::Fill).into();

            return editor::target(bare, Target::AnimFields);
        }

        let body = (size.height - KEY_HEAD_HEIGHT).max(0.0);
        let tail = if rows as f32 * FIELD_ROW_HEIGHT > body { SCROLLBAR_ALLOWANCE } else { 0.0 };
        let width = (size.width - tail).max(0.0);

        let listed = (0..rows).fold(Column::with_capacity(rows).width(Length::Fixed(width)), |listed, at| {
            match pose {
                Some(pose) => listed.push(pose.field_row(at, width)),
                None => listed,
            }
        });

        let scrolled = smooth_scroll(
            scrollable(listed)
                .id(self.fields_id.clone())
                .direction(scrollable::Direction::Vertical(bar()))
                .width(Length::Fill)
                .height(Length::Fill),
        );

        let table: Element<'_, Message> =
            column![fields_header(tail), scrolled].width(Length::Fill).height(Length::Fill).into();

        editor::target(table, Target::AnimFields)
    }

    fn absence(&self) -> &'static str {
        match self.pose.is_some() {
            true => NO_PART_CHOSEN,
            false => NO_MODEL_NOTICE,
        }
    }

    fn vacancy(&self) -> &'static str {
        let holding = self.draft.as_ref().map(|draft| draft.track.is_some());

        match (holding, self.opened.is_some()) {
            (Some(true), _) => EMPTY_TRACK_NOTICE,
            (Some(false), _) => NO_ENTRY_CHOSEN,
            (None, true) => UNREADABLE_NOTICE,
            (None, false) => NO_CLIP_NOTICE,
        }
    }
}

pub(super) fn keys_header<'a>(tail: f32) -> Element<'a, Message> {
    let row = Field::TYPED
        .iter()
        .fold(
            row![theme::centered_text("#").size(LABEL_SIZE).width(Length::Fixed(INDEX_WIDTH))].spacing(ROW_GAP),
            |header, field| header.push(theme::centered_text(field.label()).size(LABEL_SIZE).width(Length::Fill)),
        )
        .push(theme::centered_text("Curve").size(LABEL_SIZE).width(Length::Fixed(EASE_WIDTH)))
        .push(theme::centered_text("Action").size(LABEL_SIZE).width(Length::Fixed(STEP_WIDTH)))
        .push(theme::centered_text(CLOSE_LABEL).size(CELL_SIZE).width(Length::Fixed(DROP_WIDTH)));

    let inset = Padding { top: 0.0, right: KEY_ROW_INSET + tail, bottom: 0.0, left: KEY_ROW_INSET };

    container(row.width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .padding(inset)
        .style(theme::zebra_table_header)
        .into()
}

pub(super) fn adrift_input(theme: &Theme, status: text_input::Status, adrift: bool) -> text_input::Style {
    let style = theme::rounded_input(theme, status);

    if !adrift {
        return style;
    }

    let palette = theme.palette();
    let iced::Background::Color(base) = style.background else {
        return style;
    };

    let blend = |base: f32, over: f32| base * (1.0 - ADRIFT_TINT) + over * ADRIFT_TINT;
    let tinted = Color {
        r: blend(base.r, palette.danger.r),
        g: blend(base.g, palette.danger.g),
        b: blend(base.b, palette.danger.b),
        a: 1.0,
    };

    text_input::Style { background: tinted.into(), border: style.border.color(palette.danger), ..style }
}

pub(super) fn tip<'a>(content: impl Into<Element<'a, Message>>, label: &'a str) -> Element<'a, Message> {
    let banner = container(text(label).size(LABEL_SIZE))
        .padding(CELL_PADDING)
        .style(container::bordered_box);

    tooltip(content, banner, tooltip::Position::Top).into()
}

pub(super) fn console_edge<'a>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| container::Style {
            border: theme::mock_console_container(theme).border,
            ..container::Style::default()
        })
        .into()
}

pub(super) fn cuts_header<'a>(tail: f32) -> Element<'a, Message> {
    let head = CUT_FIELDS.iter().enumerate().fold(
        row![theme::centered_text("#").size(LABEL_SIZE).width(Length::Fixed(INDEX_WIDTH))].spacing(ROW_GAP),
        |listed, (cell, label)| {
            let span = match cell == CUT_NAME_FIELD {
                true => Length::Fill,
                false => Length::Fixed(CUT_CELL_WIDTH),
            };

            listed.push(theme::centered_text(*label).size(LABEL_SIZE).width(span))
        },
    );

    let head = head
        .push(theme::centered_text("Action").size(LABEL_SIZE).width(Length::Fixed(CUT_STEP_WIDTH * 2.0 + 2.0)))
        .push(theme::centered_text(CLOSE_LABEL).size(CELL_SIZE).width(Length::Fixed(DROP_WIDTH)));

    let inset = Padding { top: 0.0, right: KEY_ROW_INSET + tail, bottom: 0.0, left: KEY_ROW_INSET };

    container(head.width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .padding(inset)
        .style(theme::zebra_table_header)
        .into()
}

pub(super) fn fields_header<'a>(tail: f32) -> Element<'a, Message> {
    let row = row![
        theme::centered_text("#").size(LABEL_SIZE).width(Length::Fixed(INDEX_WIDTH)),
        theme::centered_text("Field").size(LABEL_SIZE).width(Length::Fixed(FACT_LABEL)),
        theme::centered_text("Value").size(LABEL_SIZE).width(Length::Fill),
    ]
    .spacing(ROW_GAP);

    let inset = Padding { top: 0.0, right: KEY_ROW_INSET + tail, bottom: 0.0, left: KEY_ROW_INSET };

    container(row.width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .padding(inset)
        .style(theme::zebra_table_header)
        .into()
}

pub(super) fn bar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new()
        .width(SCROLLBAR_WIDTH)
        .scroller_width(SCROLLBAR_WIDTH)
        .margin(SCROLLBAR_MARGIN)
}

pub(super) fn both_ways() -> scrollable::Direction {
    scrollable::Direction::Both { vertical: bar(), horizontal: bar() }
}

fn fact<'a>(label: &'a str, value: Element<'a, Message>, stripe: usize) -> Element<'a, Message> {
    let body = row![text(label).size(LABEL_SIZE).width(Length::Fixed(FACT_LABEL)), value]
        .spacing(ROW_GAP)
        .align_y(Vertical::Center);

    container(body)
        .width(Length::Fill)
        .padding([FACT_ROW_PAD, 6.0])
        .style(move |theme: &Theme| theme::zebra_table_row(theme, stripe))
        .into()
}

fn model_rows<'a>(index: Option<i32>, part: Option<&ModelPart>) -> Element<'a, Message> {
    let named = match (index, part) {
        (Some(index), Some(part)) => match part.name.trim() {
            "" => index.to_string(),
            name => format!("{} \u{00b7} {}", index, name),
        },
        (Some(index), None) => format!("{} \u{00b7} {}", index, NO_PART_NOTICE),
        (None, _) => String::new(),
    };

    let pair = |left: i32, right: i32| format!("{}, {}", left, right);
    let facts = [
        ("Part", named),
        ("Parent", part.map_or_else(String::new, |part| part.parent.to_string())),
        ("Sprite", part.map_or_else(String::new, |part| part.sprite.to_string())),
        ("Z Order", part.map_or_else(String::new, |part| part.z.to_string())),
        ("Offset", part.map_or_else(String::new, |part| pair(part.x, part.y))),
        ("Pivot", part.map_or_else(String::new, |part| pair(part.pivot_x, part.pivot_y))),
        ("Scale", part.map_or_else(String::new, |part| pair(part.scale_x, part.scale_y))),
        ("Opacity", part.map_or_else(String::new, |part| part.opacity.to_string())),
    ];

    facts
        .into_iter()
        .enumerate()
        .fold(column![].width(Length::Fill), |listed, (stripe, (label, value))| {
            let cell = text(value).size(LABEL_SIZE).width(Length::Fill).into();

            listed.push(fact(label, cell, stripe))
        })
        .into()
}

fn atlas_rows<'a>(index: Option<i32>, part: Option<&ModelPart>, rig: Option<&Rig>) -> Element<'a, Message> {
    let sheet = rig.map(|rig| &rig.sheet);
    let cuts = sheet.map_or(0, |sheet| sheet.cuts.len());
    let at = part.and_then(|part| usize::try_from(part.sprite).ok()).filter(|at| *at < cuts);
    let cut = at.zip(sheet).and_then(|(at, sheet)| sheet.cuts.get(at));
    let opaque = at.zip(sheet).and_then(|(at, sheet)| sheet.opaque.get(at).copied().flatten());

    let named = match (index, part) {
        (Some(index), Some(part)) => match part.name.trim() {
            "" => index.to_string(),
            name => format!("{} \u{00b7} {}", index, name),
        },
        (Some(index), None) => format!("{} \u{00b7} {}", index, LOST_PART_NOTICE),
        (None, _) => String::new(),
    };

    let sprite = match (part, at) {
        (None, _) => String::new(),
        (Some(_), Some(at)) => format!("{} of {}", at, cuts),
        (Some(part), None) if part.sprite < 0 => NO_SPRITE_NOTICE.to_owned(),
        (Some(part), None) => format!("{} \u{00b7} {}", part.sprite, PAST_ATLAS_NOTICE),
    };

    let span = |width: i32, height: i32| format!("{} \u{00d7} {}", width, height);
    let facts = [
        ("Part", named),
        ("Sprite", sprite),
        ("Cut", cut.map_or_else(String::new, |cut| format!("{}, {}", cut.x, cut.y))),
        ("Size", cut.map_or_else(String::new, |cut| span(cut.width, cut.height))),
        ("Drawn", opaque.map_or_else(|| cut.map_or_else(String::new, |_| BLANK_CUT_NOTICE.to_owned()), |seen| span(seen.width, seen.height))),
        ("Margin", margin(cut, opaque)),
        ("Region", cut.map_or_else(String::new, |cut| named_cut(&cut.name))),
        ("File", sheet.map_or_else(String::new, |sheet| sheet.image_name.clone())),
    ];

    facts
        .into_iter()
        .enumerate()
        .fold(column![].width(Length::Fill), |listed, (stripe, (label, value))| {
            let cell = text(value).size(LABEL_SIZE).width(Length::Fill).wrapping(text::Wrapping::WordOrGlyph).into();

            listed.push(fact(label, cell, stripe))
        })
        .into()
}

fn named_cut(name: &str) -> String {
    match name.trim() {
        "" => UNNAMED_CUT_NOTICE.to_owned(),
        named => named.to_owned(),
    }
}

fn margin(cut: Option<&SpriteCut>, opaque: Option<Opaque>) -> String {
    let Some(cut) = cut else {
        return String::new();
    };

    let Some(seen) = opaque else {
        return WHOLE_CUT_NOTICE.to_owned();
    };

    let left = seen.x.saturating_sub(cut.x);
    let top = seen.y.saturating_sub(cut.y);
    let right = cut.width.saturating_sub(left).saturating_sub(seen.width);
    let bottom = cut.height.saturating_sub(top).saturating_sub(seen.height);

    format!("{}, {}, {}, {}", left, top, right, bottom)
}

fn fact_header<'a>(focus: Focus) -> Element<'a, Message> {
    let label = match focus {
        Focus::Curve => "Model",
        Focus::Part => "Atlas",
    };

    container(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .style(theme::zebra_table_header)
        .into()
}

fn panel_head<'a>(label: &'a str) -> Element<'a, Message> {
    container(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .style(theme::zebra_table_header)
        .into()
}

fn dial_combo<'a, T>(held: T, options: &'a [T], live: bool, on: impl Fn(T) -> Message + 'a) -> Element<'a, Message>
where
    T: Clone + PartialEq + std::fmt::Display + 'static,
{
    sized_combo(held, options, live, COMBO_WIDTH, on)
}

fn sized_combo<'a, T>(
    held: T,
    options: &'a [T],
    live: bool,
    wide: f32,
    on: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: Clone + PartialEq + std::fmt::Display + 'static,
{
    match live {
        true => pick_list(options, Some(held), on)
            .width(Length::Fixed(wide))
            .padding([1, 4])
            .text_size(LABEL_SIZE)
            .style(theme::combo_box)
            .menu_style(theme::combo_box_menu)
            .into(),
        false => container(
            text(held.to_string())
                .size(LABEL_SIZE)
                .style(|theme: &Theme| text::Style { color: Some(theme::weak_text_color(theme)) }),
        )
        .width(Length::Fixed(wide))
        .padding([1, 4])
        .style(theme::combo_box_idle)
        .into(),
    }
}

fn dial_row<'a>(
    dial: Dial,
    readout: Readout,
    anim: &'a StudioSettings,
    animated: bool,
    stripe: usize,
) -> Element<'a, Message> {
    let live = dial.live(animated);
    let picker: Element<'_, Message> = match dial {
        Dial::Gizmo => dial_combo(anim.gizmo, &Gizmo::ALL, live, Message::Handed),
        Dial::Onion => dial_combo(anim.onion, &Switch::ALL, live, Message::Onioning),
        Dial::Module => dial_combo(readout, &Readout::ALL, live, Message::Module),
        Dial::Fault => dial_combo(anim.faults, &Faults::ALL, live, Message::Faulted),
        Dial::Entity => dial_combo(anim.entity, &Scope::ALL, live, Message::Scoped),
        _ => match dial.tier(anim) {
            Some(tier) => dial_combo(tier, &Tier::ALL, live, move |held| Message::Tiered(dial, held)),
            None => dial_combo(
                dial.shown(anim).unwrap_or_default(),
                &Shown::ALL,
                live,
                move |held| Message::Sighted(dial, held),
            ),
        },
    };

    let name = button(theme::centered_text(dial.label()).size(LABEL_SIZE).width(Length::Fill))
        .width(Length::Fixed(DEBUG_WIDTH))
        .padding([1, 4])
        .on_press_maybe(live.then_some(Message::Cycle(dial)))
        .style(theme::primary_button);

    let body = row![name, picker].spacing(ROW_GAP).align_y(Vertical::Center);

    container(body)
        .height(Length::Fixed(FACT_ROW_HEIGHT))
        .align_y(Vertical::Center)
        .padding([0, 3])
        .style(move |theme: &Theme| theme::zebra_table_row(theme, stripe))
        .into()
}

fn options<'a>(
    anim: &'a StudioSettings,
    animated: bool,
    readout: Readout,
    dials: usize,
) -> Element<'a, Message> {
    let pages = Dial::pages();
    let page = dials.min(pages.saturating_sub(1));
    let shown = Dial::page(page);

    let listed = shown
        .iter()
        .enumerate()
        .fold(column![panel_head("Option")], |listed, (stripe, dial)| {
            listed.push(dial_row(*dial, readout, anim, animated, stripe))
        });

    let padded = (shown.len()..PAGE_DIALS).fold(listed, |listed, stripe| listed.push(spare_row(stripe)));

    padded.push(pager(page, pages)).width(Length::Fixed(OPTION_WIDTH)).into()
}

fn spare_row<'a>(stripe: usize) -> Element<'a, Message> {
    container(Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(FACT_ROW_HEIGHT))
        .style(move |theme: &Theme| theme::zebra_table_row(theme, stripe))
        .into()
}

fn pager<'a>(page: usize, pages: usize) -> Element<'a, Message> {
    let step = |glyph: &'a str, onto: Option<usize>| {
        button(theme::centered_text(glyph).size(LABEL_SIZE).width(Length::Fill))
            .width(Length::Fixed(PAGER_STEP))
            .padding(0)
            .on_press_maybe(onto.map(Message::Page))
            .style(theme::pager_step)
    };

    let counted = theme::centered_text(format!("{}/{}", page + 1, pages))
        .size(LABEL_SIZE)
        .width(Length::Fill);

    let body = row![
        step(PAGE_BACK, page.checked_sub(1)),
        counted,
        step(PAGE_NEXT, (page + 1 < pages).then_some(page + 1)),
    ]
    .align_y(Vertical::Center);

    container(body)
        .width(Length::Fill)
        .height(Length::Fixed(FACT_ROW_HEIGHT))
        .align_y(Vertical::Center)
        .style(theme::zebra_table_footer)
        .into()
}

pub(super) fn console_card<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(container(body).padding(theme::CONSOLE_BORDER_WIDTH))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container)
        .into()
}

pub(super) fn centred<'a>(notice: &'a str) -> Element<'a, Message> {
    container(theme::centered_text(notice).size(LABEL_SIZE))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub(super) fn facts_card<'a>(facts: [(&'a str, String); 4]) -> Element<'a, Message> {
    let listed = facts.into_iter().enumerate().fold(
        column![].width(Length::Fill),
        |listed, (stripe, (label, value))| {
            let cell = text(value)
                .size(LABEL_SIZE)
                .width(Length::Fill)
                .wrapping(text::Wrapping::WordOrGlyph)
                .into();

            listed.push(fact(label, cell, stripe))
        },
    );

    container(container(listed).padding(theme::CONSOLE_BORDER_WIDTH))
        .width(Length::Fill)
        .style(theme::mock_console_container)
        .into()
}

pub(super) fn footer_button<'a>(label: &'a str, message: Option<Message>) -> Element<'a, Message> {
    button(theme::button_label(label).size(LABEL_SIZE))
        .width(Length::Fill)
        .padding([3, 6])
        .on_press_maybe(message)
        .style(theme::primary_button)
        .into()
}

pub(super) fn facts_table<'a>(
    focus: Focus,
    index: Option<i32>,
    part: Option<&ModelPart>,
    rig: Option<&Rig>,
) -> Element<'a, Message> {
    let table = match focus {
        Focus::Curve => model_rows(index, part),
        Focus::Part => atlas_rows(index, part, rig),
    };

    column![fact_header(focus), table].width(Length::Fill).into()
}

pub(super) fn strip<'a>(
    shown: Element<'a, Message>,
    anim: &'a StudioSettings,
    animated: bool,
    readout: Readout,
    dials: usize,
) -> Element<'a, Message> {
    let seated = container(shown).width(Length::Fill).height(Length::Fixed(timeline::BOARD_HEIGHT));
    let body = row![seated, options(anim, animated, readout, dials)].spacing(GAP);

    container(body).width(Length::Fill).into()
}

fn head_button<'a>(
    label: &'a str,
    message: Option<Message>,
    style: theme::ButtonStyleFn,
) -> Element<'a, Message> {
    button(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(HEAD_HEIGHT))
        .padding([1, 4])
        .on_press_maybe(message)
        .style(style)
        .into()
}

fn panel_head_row<'a>(mode: Mode, live: bool, shipping: Shipping) -> Element<'a, Message> {
    let other = mode.other();
    let (label, style, message) = shipping.button(live);

    let row = row![
        head_button(other.label(), Some(Message::Switch(other)), theme::primary_button),
        head_button("Manage", Some(Message::OpenManage), theme::primary_button),
        head_button(label, message, style),
    ]
    .spacing(ROW_GAP)
    .align_y(Vertical::Center);

    container(row).width(Length::Fill).height(Length::Fixed(HEAD_HEIGHT)).into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Shipping {
    Idle,
    Running,
    Placed,
    Failed,
}

impl Shipping {
    fn button(self, live: bool) -> (&'static str, theme::ButtonStyleFn, Option<Message>) {
        match self {
            Shipping::Running => ("Exporting\u{2026}", theme::warning_status, None),
            Shipping::Placed => ("Exported!", theme::success_status, None),
            Shipping::Failed => (feedback::FAILURE_LABEL, theme::danger_status, None),
            Shipping::Idle => ("Export", theme::primary_button, live.then_some(Message::Export)),
        }
    }

    pub(super) fn shipout(
        self,
        verb: &'static str,
        armed: bool,
    ) -> (&'static str, theme::ButtonStyleFn, Option<Message>) {
        match (self, armed) {
            (Shipping::Idle, true) => (feedback::CONFIRM_LABEL, theme::danger_button, Some(Message::Ship)),
            (Shipping::Idle, false) => (verb, theme::primary_button, Some(Message::Ship)),
            _ => self.button(false),
        }
    }
}

pub(super) fn panel_frame<'a>(
    mode: Mode,
    live: bool,
    shipping: Shipping,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let framed = column![panel_head_row(mode, live, shipping), rule::horizontal(1), body]
        .spacing(ROW_GAP)
        .height(Length::Fill);

    container(framed)
        .width(Length::Fixed(PANEL_WIDTH))
        .height(Length::Fill)
        .padding(PANEL_PADDING)
        .into()
}

pub(super) fn vacant_cuts<'a>() -> Element<'a, Message> {
    facts_card([
        ("Sheet", String::new()),
        ("Cuts", String::new()),
        ("Size", String::new()),
        ("Regions", String::new()),
    ])
}

pub(super) fn vacant_slices<'a>() -> Element<'a, Message> {
    let table = console_card(column![cuts_header(0.0), centred(NO_SET_NOTICE)].width(Length::Fill));

    column![table, footer_button("Add Cut", None)].spacing(ROW_GAP).height(Length::Fill).into()
}

pub(super) fn vacant_keys<'a>() -> Element<'a, Message> {
    let table = console_card(column![keys_header(0.0), centred(NO_SET_HINT)].width(Length::Fill));

    column![table, footer_button("Add Keyframe", None)].spacing(ROW_GAP).height(Length::Fill).into()
}
