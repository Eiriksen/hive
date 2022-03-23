use crate::request::game::*;
use seed::{self, prelude::*, *};
use shared::model::ResponseBody;
use shared::{model::game::*, ObjectId};
use web_sys::Event;
use web_sys::SvgGraphicsElement;

use crate::page::game_util::*;

#[derive(Default)]
pub struct Model {
    pub game: Option<GameResource>,
    pub gridv3: Vec<Hex>,
    pub piece: Option<SelectedPiece>,
    pub menu: Option<Vec<MenuItem>>,
    pub svg: ElRef<SvgGraphicsElement>,
    pub color: Option<Color>,

    pub size: String,
    pub label: Option<String>,
}

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Option<Model> {
    let gen_size = |n: f32| {
        let l = 5. * n;
        let h = 9. * n;
        let w = 10. * n;

        format!("{l}, -{h} -{l}, -{h} -{w}, 0 -{l}, {h} {l}, {h} {w}, 0")
    };
    match url.next_path_part() {
        Some(id) => match ObjectId::parse_str(id) {
            Ok(id) => {
                orders.perform_cmd(async move { Msg::FetchGame(get_game(id).await) });
                let size = gen_size(0.5);

                Some(Model {
                    game: None,
                    gridv3: create_gridv3(4),
                    menu: None,
                    svg: ElRef::default(),
                    color: None,
                    piece: None,
                    size,
                    label: None,
                })
            }
            _ => None,
        },
        _ => None,
    }
}

pub enum Msg {
    FetchGame(fetch::Result<String>),
    SentMove(fetch::Result<String>),

    Move(Event),
    Click(Event),
    Release(Event),
    Place((String, Event)),
    Drag(Piece),
    MouseUp(Event),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    let parse_resp = |resp: fetch::Result<String>| -> Result<ResponseBody, String> {
        resp.map_err(|e| format!("{e:?}"))
            .and_then(|text| {
                serde_json::from_str::<ResponseBody>(&text).map_err(|e| format!("{e:?}"))
            })
            .and_then(|resp| {
                if resp.status == 200 {
                    Ok(resp)
                } else {
                    Err("wrong statuscode".into())
                }
            })
    };
    match msg {
        Msg::SentMove(resp) => {
            if let Err(e) = parse_resp(resp) {
                model.label = Some(format!("{e:?}"));
            }
        }

        Msg::FetchGame(res) => match parse_resp(res) {
            Ok(resp) => {
                let game: GameResource = resp.get_body();
                model.color = get_color(&game);
                model.game = Some(game);
                if let Some(color) = model.color {
                    model.menu = Some(create_menu(color));
                }
                log(legal_turn(model));

                grid_from_board(model);
            }
            Err(e) => {
                model.label = Some(format!("expected 200 got {e}"));
            }
        },

        Msg::Click(event) => {
            let mm = to_mouse_event(&event);
            if mm.button() != 0 {
                return;
            }
            let (x, y) = get_mouse_pos(model, mm);
            let sq = pixel_to_hex(x as isize, y as isize);

            log(sq);

            if let Some(hex) = get_piece_from_square_mut(model, sq) {
                let cl = hex.clone();
                hex.remove_top();

                let mut sel: SelectedPiece = cl.into();
                sel.x = x;
                sel.y = y;
                model.piece = Some(sel);
            }
            // clear_red(model);
        }

        Msg::Release(event) => {
            let mm = to_mouse_event(&event);
            let (x, y) = get_mouse_pos(model, mm);
            let sq = pixel_to_hex(x as isize, y as isize);

            if mm.button() != 0 {
                return;
            }

            if let Some(selected_piece) = model.piece.take() {
                if legal_move(model, sq) {
                    // Place the piece
                    get_hex_from_square(model, sq)
                        .unwrap()
                        .place_piece(selected_piece.piece);

                    let board = get_board_mut(model).unwrap();
                    board.place_piece(selected_piece.piece, sq, Some(selected_piece.old_square));

                    if let Some(r#move) = get_move(
                        model,
                        selected_piece.piece,
                        sq,
                        Some(selected_piece.old_square),
                    ) {
                        orders.perform_cmd(async move { Msg::SentMove(send_move(r#move).await) });
                    }
                } else {
                    place_piece_back(model, selected_piece);
                }

                clear_highlighs(model);
                //clear_red(model);
            }
        }

        Msg::Move(event) => {
            let mm = to_mouse_event(&event);
            let (x, y) = get_mouse_pos(model, mm);

            let my_turn = legal_turn(model);
            let correct_piece = legal_piece(model);

            if let Some(sel) = model.piece.as_mut() {
                sel.x = x;
                sel.y = y;

                if my_turn && correct_piece {
                    let piece = &sel.piece;
                    let board = &model.game.as_ref().unwrap().board;

                    let legal_moves = legal_moves(piece, board, Some(sel.old_square));
                    set_highlight(model, legal_moves, true);
                }
            }
        }

        Msg::Place((id, event)) => {
            let mm = to_mouse_event(&event);
            let (x, y) = get_mouse_pos(model, mm);
            let sq = pixel_to_hex(x as isize, y as isize);


            let r#type: BoardPiece = id.into();
            // Todo fix
            let color = if model.game.as_ref().unwrap().board.turns % 2 == 0 {
                Color::White
            } else {
                Color::Black
            };

            let piece = Piece { r#type, color };
            if legal_move(model, sq) {
                place_piece(model, piece, sq);
                if let Some(ref mut b) = get_board_mut(model) {
                    b.place_piece(piece, sq, None);
                }
                if let Some(r#move) = get_move(model, piece, sq, None) {
                    orders.perform_cmd(async move { Msg::SentMove(send_move(r#move).await) });
                }
            }

            clear_highlighs(model);
        }

        Msg::Drag(piece) => {
            log(legal_turn(model));
            if legal_turn(model) {
                let board = get_board(model).unwrap();
                let legal_moves = legal_moves(&piece, board, None);
                log(&legal_moves);
                set_highlight(model, legal_moves, true);
            }
        }

        Msg::MouseUp(event) => {
            let mm = to_mouse_event(&event);
            // Secondary button, i.e, right-click
            if mm.button() == 2 {
                let (x, y) = get_mouse_pos(model, mm);
                let sq = pixel_to_hex(x as isize, y as isize);

                if let Some(hex) = get_hex_from_square(model, sq) {
                    hex.red = !hex.red;
                }
            }
            // main button, i.e, left-click
            // should this not be 1?
            // https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/buttons
            else if mm.button() == 0 {
                //clear_red(model);
            }
        }
    }
}

pub fn view(model: &Model) -> Node<crate::Msg> {
    div![div![
        C!("container"),
        grid(model),
        IF!(model.color.is_some() =>
            div![C!("piece-menu"), piece_menu(model)]
        ),
        IF!(model.label.is_some() => match model.label {
            Some(ref s) => h2! [C!("error"), s],
            _ => unreachable!()
        })
    ]]
}

pub struct MenuItem {
    x: f32,
    y: f32,
    piece: Piece,
    //spacing: f32,
}

impl MenuItem {
    fn to_menu_node(&self) -> Node<crate::Msg> {
        let (x, y) = (self.x, self.y);

        let stroke = piece_color(self.piece.r#type, self.piece.color);
        let id = format!("{:?}", self.piece.r#type);

        let piece_clone = self.piece;
        div![
            ev(Ev::DragStart, move |event| {
                let ev = to_drag_event(&event);
                use web_sys::{Element, HtmlDivElement};

                let idv = event
                    .current_target()
                    .unwrap()
                    .dyn_ref::<HtmlDivElement>()
                    .unwrap()
                    .clone();

                let el: &Element = idv.as_ref();
                let id = el.id();
                // This is a big yikes; however, good enough for now.

                ev.data_transfer()
                    .unwrap()
                    .set_data("text/plain", &id)
                    .unwrap();


                crate::Msg::Game(Msg::Drag(piece_clone))
            }),
            id!(&id),
            style! {
                    St::Width => "90px",//self.spacing,
                    St::Height => "90px", // ??
                    St::Background => stroke,
            },
            attrs! {
                    At::X => x,
                    At::Y => y,
                    At::Draggable => "true",
            }
        ]
    }
}

fn create_menu(color: Color) -> Vec<MenuItem> {
    let deltas = |n: f32| (15. * n, 9. * n);
    let (_, dy) = deltas(0.5);

    use BoardPiece::*;
    let colors = [Queen, Ant, Spider, Grasshopper, Beetle];
    let _spacing = 100.0 / colors.len() as f32;


    // Should be set to the players colors
    let len = colors.len();

    colors
        .into_iter()
        .enumerate()
        .map(|(i, bp)| {
            let piece = Piece { r#type: bp, color };
            let size = 100.0 / len as f32;
            let per = size / 2.0; // center
            let x = ((i + 1) as f32 * size) - per;

            let y = 1.65 * dy; // ???

            MenuItem { x, y, piece }
        })
        .collect()
}

fn piece_menu(model: &Model) -> Node<crate::Msg> {
    if model.menu.is_none() {
        div![style! {
            St::Display => "flex",
        }]
    } else {
        div![
            style! {
                St::Display => "flex",
            },
            model
                .menu
                .as_ref()
                .unwrap()
                .iter()
                .map(MenuItem::to_menu_node)
        ]
    }
}

pub fn grid(model: &Model) -> Node<crate::Msg> {
    div![
        ev(Ev::Drop, |event| {
            let ev = to_drag_event(&event);
            let id = ev.data_transfer().unwrap().get_data("text/plain").unwrap();
            crate::Msg::Game(Msg::Place((id, event)))
        }),
        ev(Ev::DragOver, |event| {
            //log("DragOver");
            event.prevent_default();
        }),
        ev(Ev::ContextMenu, |event| {
            event.prevent_default();
        }),
        ev(Ev::MouseUp, |event| {
            crate::Msg::Game(Msg::MouseUp(event))
        }),
        svg![
            el_ref(&model.svg),
            ev(Ev::MouseMove, |event| {
                crate::Msg::Game(Msg::Move(event))
            }),
            ev(Ev::MouseUp, |event| {
                event.prevent_default();
                crate::Msg::Game(Msg::Release(event))
            }),
            ev(Ev::MouseDown, |event| {
                crate::Msg::Game(Msg::Click(event))
            }),
            attrs! {
                At::ViewBox => "0 0 100 100",
                At::Draggable => "true",
            },
            defs![g![
                attrs! { At::Id => "pod" },
                polygon![attrs! {
                    //At::Stroke => "gold",
                    At::StrokeWidth => ".5",
                    At::Points => &model.size,
                },]
            ]],
            model.gridv3.iter().map(Hex::node),
            IF!(model.piece.is_some() => {
                model.piece.as_ref().unwrap().node()
            })
        ]
    ]
}

/// HEX STUFF
struct Orientation {
    f0: f32,
    f1: f32,
    f2: f32,
    f3: f32,

    _b0: f32,
    _b1: f32,
    _b2: f32,
    _b3: f32,

    _start_angle: f32,
}

impl Orientation {
    fn flat() -> Self {
        Self {
            f0: 3.0 / 2.0,
            f1: 0.0,
            f2: 3.0_f32.sqrt() / 2.0,
            f3: 3.0_f32.sqrt(),

            _b0: 2.0 / 3.0,
            _b1: 0.0,
            _b2: -1.0 / 3.0,
            _b3: 3.0_f32.sqrt() / 3.0,
            _start_angle: 0.0,
        }
    }
}

fn round(_q: f32, _r: f32, _s: f32) -> Square {
    let mut q = _q.round();
    let mut r = _r.round();
    let mut s = _s.round();

    let q_diff = (q - _q).abs();
    let r_diff = (r - _r).abs();
    let s_diff = (s - _s).abs();

    if q_diff > r_diff && q_diff > s_diff {
        q = -r - s;
    } else if r_diff > s_diff {
        r = -q - s;
    } else {
        s = -q - r;
    }
    (q as isize, r as isize, s as isize)
}

#[allow(non_snake_case)]
pub fn pixel_to_hex(x: isize, y: isize) -> Square {
    let (x, y) = (x as f32 - 50., y as f32 - 50.);
    const S: f32 = 5.1;

    let x = x / S;
    let y = y / S;


    let q = 2.0 / 3.0 * x;
    let r = (-1.0 / 3.0) * x + (3.0_f32.sqrt() / 3.0) * y;

    let s = -q - r;

    round(q, r, s)
}

#[derive(Clone)]
pub struct Hex {
    pub q: isize,
    pub r: isize,
    pub s: isize,

    pub _x: f32,
    pub _y: f32,

    pub pieces: Vec<Piece>,
    pub selected: bool,

    pub highlight: bool,
    pub red: bool,
}

impl Hex {
    pub fn new(q: isize, r: isize, s: isize) -> Self {
        Self {
            q,
            r,
            s,
            _x: 0.,
            _y: 0.,
            pieces: Vec::new(),
            selected: false,
            highlight: false,
            red: false,
        }
    }

    pub fn top(&self) -> Option<&Piece> {
        self.pieces.last()
    }

    pub fn place_piece(&mut self, piece: Piece) {
        self.pieces.push(piece);
    }

    pub fn remove_top(&mut self) -> Option<Piece> {
        self.pieces.pop()
    }


    #[allow(non_snake_case)]
    fn to_pixels(&self) -> (f32, f32) {
        let M = Orientation::flat();
        const S: f32 = 5.1;

        let x: f32 = (M.f0 * self.q as f32 + M.f1 * self.r as f32) * S;
        let y: f32 = (M.f2 * self.q as f32 + M.f3 * self.r as f32) * S;

        (x + 50.0, y + 50.0)
    }

    pub fn sq(&self) -> Square {
        (self.q, self.r, self.s)
    }


    fn node(&self) -> Node<crate::Msg> {
        let (x, y) = self.to_pixels();

        let opacity = match self.selected {
            true => "0.5",
            false => "1.0",
        };



        // @NOTE: red highlight removes any other color,
        // Improve this when using actual images.
        let fill = match (self.red, self.top(), self.selected) {
            (true, _, _) => "red",
            (_, Some(p), _) => piece_color(p.r#type, p.color),
            (_, None, false) => "transparent",
            (_, None, true) => "grey",
        };

        let c = if self.highlight { "selected-piece" } else { "" };

        r#use![attrs! {
            At::Href => "#pod",
            At::Transform => format!("translate({x}, {y})"),
            At::Fill => fill,
            At::Stroke => "gold",
            At::Opacity => opacity,
            At::Class => c,
            At::DropZone => "move",
        },]
    }
}

fn create_gridv3(r: usize) -> Vec<Hex> {
    use std::cmp::{max, min};
    let r = r as isize;

    let mut vec = Vec::new();
    for q in -r..=r {
        let r1 = max(-r, -q - r);
        let r2 = min(r, -q + r);

        for r in r1..=r2 {
            vec.push(Hex::new(q, r, -q - r));
        }
    }
    vec
}
