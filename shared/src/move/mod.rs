use crate::model::*;

mod ant;
use ant::*;

mod beetle;
use beetle::*;

mod spider;
use spider::*;

mod queen;
use queen::*;

mod grasshopper;
use grasshopper::*;

pub fn legal_moves(p: &Piece, board: &mut Board, board_pos: Option<Square>) -> Vec<Square>
{
    if board.is_complete()
    {
        return Vec::new();
    }

    match board.turns
    {
        // These first we _know_ and can be hardcoded
        0 => vec![(0, 0, 0)],
        1 => vec![(1, -1, 0), (-1, 1, 0), (1, 0, -1), (-1, 0, 1), (0, 1, -1), (0, -1, 1)],

        _ =>
        {
            let idx = p.color as usize;

            let queen_turn = board.turns == 6 || board.turns == 7;
            let no_queen_placed = board.queens[idx].is_none();
            let piece_not_queen = p.r#type != BoardPiece::Queen;

            // A queen _has_ to be placed in the first four move of each player
            if queen_turn && no_queen_placed && piece_not_queen
            {
                return Vec::new();
            }

            match board_pos
            {
                Some(pos) => legal_on_board_move(p, board, pos),
                None => legal_new_piece_moves(p, board),
            }
        },
    }
}


pub fn square_has_neighbors(sq: Square, board: &Board, me: Square) -> bool
{
    neighbors(&sq).into_iter().filter(|s| *s != me).any(|s| board.contains_key(&s))
}


fn legal_on_board_move(p: &Piece, board: &mut Board, sq: Square) -> Vec<Square>
{
    let vec = match p.r#type
    {
        BoardPiece::Ant => ant_move(board, sq),
        BoardPiece::Beetle => beetle_move(board, sq),
        BoardPiece::Grasshopper => grasshopper_move(board, sq),
        BoardPiece::Queen => queen_move(board, sq),
        BoardPiece::Spider => spider_move(board, sq),
    };
    create_island_multiple(board, sq, vec)
}

// Hmm, t-this can be simplified r-right?
fn legal_new_piece_moves(piece: &Piece, board: &Board) -> Vec<Square>
{
    // Good neighbors have only same color neighbors or none
    let good_neighbors = |sq: &Square| {
        neighbors(sq).into_iter().all(|sq| match board.get(&sq)
        {
            None => true,
            Some(s) => s.top().color == piece.color,
        })
    };

    let not_touching_other_color =
        //|sq: Square| board.board.get(&sq).map_or(true, |s| s.piece.color == piece.color);
        |sq: Square| board.get(&sq).is_none() && good_neighbors(&sq);

    board
        .iter()
        .filter_map(|(sq, bp)| {
            (bp.top().color == piece.color).then(|| {
                neighbors(sq)
                    .into_iter()
                    .filter_map(|sq| not_touching_other_color(sq).then(|| sq))
            })
        })
        .flatten()
        .collect()
}

pub fn neighbors(sq: &Square) -> [Square; 6]
{
    const CUBE_DIR_VEC: [(isize, isize, isize); 6] =
        [(1, 0, -1), (1, -1, 0), (0, -1, 1), (-1, 0, 1), (-1, 1, 0), (0, 1, -1)];

    let mut iter = CUBE_DIR_VEC.into_iter().map(|d| (sq.0 + d.0, sq.1 + d.1, sq.2 + d.2));

    [
        iter.next().unwrap(),
        iter.next().unwrap(),
        iter.next().unwrap(),
        iter.next().unwrap(),
        iter.next().unwrap(),
        iter.next().unwrap(),
    ]
}


fn create_island_multiple(board: &Board, from: Square, mut vec: Vec<Square>) -> Vec<Square>
{
    let mut global = Vec::with_capacity(board.len());
    let mut local = Vec::with_capacity(board.len());

    vec.retain(|to| !_create_island(board, from, *to, &mut global, &mut local));
    vec
}

pub fn _create_island(
    board: &Board,
    from: Square,
    to: Square,
    global: &mut Vec<Square>,
    local: &mut Vec<Square>,
) -> bool
{
    let mut board = board.clone();
    board.play_from_to(from, to);

    let mut iter = neighbors(&from)
        .into_iter()
        .filter(|sq| match board.get(sq)
        {
            Some(bs) => !bs.pieces.is_empty(),
            _ => false,
        })
        .chain(std::iter::once(to));

    let res = if let Some(fst) = iter.next()
    {
        //if global.is_empty()
        {
            global.clear();
            create_set(&board, fst, global);
        }

        iter.any(|s| {
            local.clear();
            check_global(&board, s, &global, local)
        })
    }
    else
    {
        false
    };

    //board.un_play_from_to(from, to);
    res
}


// @TODO, make this better
fn create_set(board: &Board, fst: Square, set: &mut Vec<Square>)
{
    for sq in neighbors(&fst).into_iter().filter(|sq| match board.get(sq)
    {
        Some(bs) => !bs.pieces.is_empty(),
        _ => false,
    })
    {
        if !set.contains(&sq)
        //if set.insert(sq)
        {
            set.push(sq);
            create_set(board, sq, set);
        }
    }
}

fn check_global(board: &Board, sq: Square, global: &Vec<Square>, local: &mut Vec<Square>) -> bool
{
    if !global.contains(&sq)
    {
        return true;
    }

    //for sq in neighbors(&sq).into_iter().filter(|s| board.board.contains_key(s))
    for sq in neighbors(&sq).into_iter().filter(|sq| match board.get(sq)
    {
        Some(bs) => !bs.pieces.is_empty(),
        _ => false,
    })
    {
        if !local.contains(&sq)
        //if local.push(sq)
        {
            local.push(sq);
            if check_global(board, sq, global, local)
            {
                return true;
            }
        }
    }
    false
}


pub fn create_island(board: &mut Board, from: Square, to: Square) -> bool
{
    let mut board = board.clone();
    board.play_from_to(from, to);

    let mut iter = neighbors(&from)
        .into_iter()
        .filter(|sq| match board.get(sq)
        {
            Some(bs) => !bs.pieces.is_empty(),
            _ => false,
        })
        .chain(std::iter::once(to));

    let res = if let Some(fst) = iter.next()
    {
        let mut global = Vec::with_capacity(board.len());
        let mut local = Vec::with_capacity(board.len());


        create_set(&board, fst, &mut global);

        iter.any(|s| {
            local.clear();
            check_global(&board, s, &global, &mut local)
        })
    }
    else
    {
        false
    };

    //board.un_play_from_to(from, to);
    res
}

#[cfg(test)]
mod test
{
    use super::*;

    #[test]
    fn test_get_correct_neighbors()
    {
        let same = |a: [Square; 6], b: [Square; 6]| a.iter().all(|a| b.contains(a));

        assert!(same(neighbors(&(0, 0, 0)), [
            (0, -1, 1),
            (0, 1, -1),
            (1, 0, -1),
            (-1, 0, 1),
            (1, -1, 0),
            (-1, 1, 0)
        ]));

        assert!(same(neighbors(&(0, -2, 2)), [
            (0, -3, 3),
            (1, -3, 2),
            (1, -2, 1),
            (0, -1, 1),
            (-1, -1, 2),
            (-1, -2, 3)
        ]));
    }

    #[test]
    fn can_detect_create_islands_simple()
    {
        let mut board = Board::default();
        for sq in [(0, -1, 1), (0, 0, 0), (0, 1, -1)]
        {
            board.insert(sq, BoardSquare::new(Piece::new(BoardPiece::Ant, Color::Black)));
        }

        let from = (0, 1, -1);
        let to = (1, 0, -1);

        assert!(!create_island(&mut board, from, to));

        let from = (0, 1, -1);
        let to = (0, 2, -2);

        assert!(create_island(&mut board, from, to));
    }

    #[test]
    fn can_detect_create_islands_more_pieces()
    {
        let mut board = Board::default();


        let squares = [
            (0, -1, 1),
            (0, 0, 0),
            (0, 1, -1),
            (0, 2, -2),
            (-2, 0, 2),
            (-2, 1, 1),
            (-1, 0, 1),
            (-1, 1, 0),
            (-1, 3, -2),
            (1, -1, 0),
            (1, 0, -1),
            (1, 1, -2),
            (1, 2, -3),
        ];


        for sq in squares
        {
            board.insert(sq, BoardSquare::new(Piece::new(BoardPiece::Ant, Color::Black)));
        }

        let from = (1, 0, -1);
        let to = (2, -1, -1);

        assert!(!create_island(&mut board, from, to));

        let from = (1, 0, -1);
        let to = (3, -1, -2);

        assert!(create_island(&mut board, from, to));
    }

    #[test]
    fn can_detect_create_island_circle()
    {
        let mut board = Board::default();


        let squares = [
            (-1, -2, 3),
            (-1, -1, 2),
            (-1, 0, 1),
            (-1, 1, 0),
            (0, 1, -1),
            (1, 1, -2),
            (2, 1, -3),
            (3, 0, -3),
            (4, -1, -3),
            (4, -2, -2),
            (4, -3, -1),
            (4, -4, 0),
            (3, -4, 1),
            (2, -4, 2),
            (1, -4, 3),
            (0, -3, 3),
            (0, 0, 0),
        ];


        for sq in squares
        {
            board.insert(sq, BoardSquare::new(Piece::new(BoardPiece::Ant, Color::Black)));
        }

        let from = (0, 0, 0);
        let to = (1, 0, -1);

        assert!(!create_island(&mut board, from, to));

        let from = (-1, -2, 3);
        let to = (-2, -1, 3);

        assert!(!create_island(&mut board, from, to));

        board.remove((2, 1, -3));
        let from = (-1, -2, 3);
        let to = (-2, -1, 3);

        assert!(create_island(&mut board, from, to));
    }
}
