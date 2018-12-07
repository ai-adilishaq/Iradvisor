use std::{
	collections::HashSet,
	io,
	fs::File,
	fmt,
	error::Error,
};

pub type Cell = u32;
pub type Grid = Vec<Vec<Cell>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridType {
	Finite,		// Finite rectangular grid with sink all around the grid.
	Toroidal,	// Toroidal rectangular grid with sink at the top-left node.
}

#[derive(Debug, Clone)]
pub struct GridSandpile {
	grid_type: GridType,
	grid: Grid,
	last_topple: u64,
}

impl PartialEq for GridSandpile {
	fn eq(&self, other: &GridSandpile) -> bool {
		self.grid_type == other.grid_type && self.grid == other.grid
	}
}

impl Eq for GridSandpile {}

impl fmt::Display for GridSandpile {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let vis = [" ", ".", ":", "&", "#"];
		for row in &self.grid {
			for el in row {
				write!(f, "{}", vis[if *el < 4 {*el} else {4} as usize])?;
			}
			writeln!(f)?;
		}
		Ok(())
	}
}

impl GridSandpile {
	pub fn from_grid(grid_type: GridType, grid: Grid) -> Result<GridSandpile, SandpileError> {
		if grid.is_empty() {
			return Err(SandpileError::EmptyGrid);
		}
		let l = grid[0].len();
		if l == 0 {
			return Err(SandpileError::EmptyFirstRow(grid));
		}
		for i in 1..grid.len() {
			let l2 = grid[i].len();
			if l2 != l {
				return Err(SandpileError::UnequalRowLengths(grid, l, i, l2));
			}
		}
		let mut sandpile = GridSandpile {
			grid_type,
			grid,
			last_topple: 0,
		};
		if grid_type == GridType::Toroidal {
			sandpile.grid[0][0] = 0;
		}
		sandpile.topple();
		Ok(sandpile)
	}

	pub fn from_string(grid_type: GridType, (x, y): (usize, usize), s: String) -> Result<GridSandpile, SandpileError> {
		let mut g = Vec::new();
		for line in s.lines() {
			let mut row = Vec::new();
			for ch in line.chars() {
				row.push(match ch {
					' ' => 0,
					'.' => 1,
					':' => 2,
					'&' => 3,
					'#' => 4,
					_ => return Err(SandpileError::UnknownSymbol(ch))
				});
			}
			g.push(row);
		}
		if y == 0 || x == 0 || g.len() == 0 {
			return Err(SandpileError::EmptyGrid);
		}
		let s = GridSandpile::from_grid(grid_type, g)?;
		if s.grid.len() != y || s.grid[0].len() != x {
			return Err(SandpileError::UnequalDimensions(x, y, s.grid.len(), s.grid[0].len()))
		}
		Ok(s)
	}

	pub fn add(&mut self, p: &GridSandpile) -> Result<(), SandpileError> {
		if p.grid_type != self.grid_type {
			return Err(SandpileError::UnequalTypes(self.grid_type, p.grid_type));
		}
		if p.grid.len() != self.grid.len() || p.grid[0].len() != self.grid[0].len() {
			return Err(SandpileError::UnequalDimensions(
			self.grid.len(), self.grid[0].len(), p.grid.len(), p.grid[0].len()));
		}
		for i in 0..self.grid.len() {
			for j in 0..self.grid[0].len() {
				self.grid[i][j] += p.grid[i][j];
			}
		}
		self.topple();
		Ok(())
	}
	
	pub fn neutral(grid_type: GridType, (x, y): (usize, usize)) -> GridSandpile {
	// Proposition 6.36 of http://people.reed.edu/~davidp/divisors_and_sandpiles/
		let mut sandpile = GridSandpile::from_grid(grid_type, vec![vec![6; x]; y]).unwrap();
		for row in &mut sandpile.grid {
			for el in row {
				*el = 6 - *el;
			}
		}
		if grid_type == GridType::Toroidal {
			sandpile.grid[0][0] = 0;
		}
		sandpile.topple();
		sandpile
	}

	pub fn into_grid(self) -> Grid {
		self.grid
	}

	fn topple(&mut self) -> u64 {
		let mut excessive = HashSet::new();
		let mut ex2;
		for i in 0..self.grid.len() {
			for j in 0..self.grid[i].len() {
				if self.grid[i][j] >= 4 {
					excessive.insert((i, j));
				}
			}
		}
		let mut count = 0;
		while !excessive.is_empty() {
			ex2 = HashSet::new();
			for c in excessive.drain() {
				let (i, j) = c;
				let d = self.grid[i][j] / 4;
				if d == 0 {
					continue;
				}
				self.grid[i][j] %= 4;
				count += d as u64;
				let mut topple_to = Vec::new();
				match self.grid_type {
					GridType::Finite => {
						if i > 0 {
							topple_to.push((i-1, j));
						}
						if j > 0 {
							topple_to.push((i, j-1));
						}
						if i < self.grid.len()-1 {
							topple_to.push((i+1, j));
						}
						if j < self.grid[i].len()-1 {
							topple_to.push((i, j+1));
						}
					},
					GridType::Toroidal => {
						let i1 = if i > 0 {i-1} else {self.grid.len()-1};
						if !(i1 == 0 && j == 0) {
							topple_to.push((i1, j));
						}
						let j1 = if j > 0 {j-1} else {self.grid[0].len()-1};
						if !(i == 0 && j1 == 0) {
							topple_to.push((i, j1));
						}
						let i1 = if i < self.grid.len()-1 {i+1} else {0};
						if !(i1 == 0 && j == 0) {
							topple_to.push((i1, j));
						}
						let j1 = if j < self.grid[i].len()-1 {j+1} else {0};
						if !(i == 0 && j1 == 0) {
							topple_to.push((i, j1));
						}
					},
				};
				for (ti, tj) in topple_to {
					self.grid[ti][tj] += d;
					if self.grid[ti][tj] >= 4 {
						ex2.insert((ti, tj));
					}
				}
			}
			excessive = ex2;
		}
		self.last_topple = count;
		count
	}
	
	pub fn last_topple(&self) -> u64 {
		self.last_topple
	}
	
	pub fn inverse(&self) -> GridSandpile {
		let mut sandpile = GridSandpile::from_grid(self.grid_type, vec![vec![6; self.grid[0].len()]; self.grid.len()]).unwrap();
		for y in 0..self.grid.len() {
			for x in 0..self.grid[0].len() {
				sandpile.grid[y][x] = 2 * (6 - sandpile.grid[y][x]) - self.grid[y][x];
			}
		}
		if self.grid_type == GridType::Toroidal {
			sandpile.grid[0][0] = 0;
		}
		sandpile.topple();
		sandpile
	}

	pub fn order(&self) -> u64
	{
		let mut a = self.clone();
		a.add(self).unwrap();
		let mut count = 1;
		while a != *self {
			a.add(self).unwrap();
			count += 1;
		}
		count
	}
	
	pub fn grid_type(&self) -> GridType {
		self.grid_type
	}
}

#[derive(Debug)]
pub enum SandpileError {
	EmptyGrid,
	EmptyFirstRow(Grid),
	UnequalRowLengths(Grid, usize, usize, usize),
	UnequalTypes(GridType, GridType),
	UnequalDimensions(usize, usize, usize, usize),
	UnknownSymbol(char),
}

impl fmt::Display for SandpileError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			SandpileError::EmptyGrid => write!(f, "Attempt to build a sandpile upon zero-size grid."),
			SandpileError::EmptyFirstRow(_) => write!(f, "Sandpile grid has empty initial row."),
			SandpileError::UnequalRowLengths(_, expected, n, got) =>
				write!(f, "Sandpile grid does not represent rectangular matrix: initial row has length {}, row {} has length {}.",
					expected, n, got),
			SandpileError::UnequalTypes(expected, got) =>
				write!(f, "Adding sandpiles on grids of different types: {:?} and {:?}.", expected, got),
			SandpileError::UnequalDimensions(self_x, self_y, other_x, other_y) =>
				write!(f, "Incorrect dimensions of sandpile grids: expected {}x{}, got {}x{}.",
					self_x, self_y, other_x, other_y),
			SandpileError::UnknownSymbol(ch) => write!(f, "Unknown symbol in the text representation of a sandpile: {}", ch),
		}
	}
}

impl Error for SandpileError {}

impl SandpileError {
	pub fn into_grid(self) -> Option<Grid> {
		match self {
			SandpileError::EmptyFirstRow(grid)
			| SandpileError::UnequalRowLengths(grid, ..) =>
				Some(grid),
			_ => None,
		}
	}
}

pub fn png(grid: &Grid, fname: &str) -> io::Result<()> {
	let colors = [
		[0, 0, 0, 255],
		[64, 128, 0, 255],
		[118, 8, 170, 255],
		[255, 214, 0, 255],
	];
	let mut pixels = vec![0; grid.len() * grid[0].len() * 4];
	let mut p = 0;
	for row in grid {
		for el in row {
			pixels[p..p+4].copy_from_slice(&colors[*el as usize]);
			p += 4;
		}
	}
	repng::encode(File::create(fname)?, grid[0].len() as u32, grid.len() as u32, &pixels)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn id_finite() {
		let s = GridSandpile::neutral(GridType::Finite, (3, 2));
		let g = s.into_grid();
		assert_eq!(g, vec![vec![2, 1, 2], vec![2, 1, 2]]);
	}
	
	#[test]
	fn id_torus() {
		let s = GridSandpile::neutral(GridType::Toroidal, (3, 2));
		let g = s.into_grid();
		assert_eq!(g, vec![vec![0, 3, 3], vec![2, 1, 1]]);
	}
	
	#[test]
	fn from_string() {
		let st = "&. \n:.:\n";
		let s = GridSandpile::from_string(GridType::Finite, (3, 2), String::from(st)).unwrap();
		let g = s.into_grid();
		assert_eq!(g, vec![vec![3, 1, 0], vec![2, 1, 2]]);
		let s = GridSandpile::from_string(GridType::Toroidal, (3, 2), String::from(st)).unwrap();
		let g = s.into_grid();
		assert_eq!(g, vec![vec![0, 1, 0], vec![2, 1, 2]]);
	}
	
	#[test]
	fn display() {
		let g = vec![vec![3, 1, 0], vec![2, 1, 2]];
		let s = GridSandpile::from_grid(GridType::Finite, g.clone()).unwrap();
		assert_eq!(format!("{}", s), String::from("&. \n:.:\n"));
		let s = GridSandpile::from_grid(GridType::Toroidal, g).unwrap();
		assert_eq!(format!("{}", s), String::from(" . \n:.:\n"));
	}
	
	#[test]
	fn add() {
		let mut s1 = GridSandpile::from_grid(GridType::Finite, vec![vec![2, 1, 2], vec![3, 3, 1], vec![2, 3, 1]]).unwrap();
		let r = s1.clone();
		let s2 = GridSandpile::from_grid(GridType::Finite, vec![vec![2, 1, 2], vec![1, 0, 1], vec![2, 1, 2]]).unwrap();
		s1.add(&s2).unwrap();
		assert_eq!(s1, r);
		assert_eq!(r.last_topple(), 0);
		assert_eq!(s1.last_topple(), 9);
	}
	
	#[test]
	fn order() {
		let s = GridSandpile::from_grid(GridType::Finite, vec![vec![3, 3, 3], vec![3, 3, 3]]).unwrap();
		assert_eq!(s.order(), 7);
	}
	
	#[test]
	fn inverse() {
		let s = GridSandpile::from_grid(GridType::Finite, vec![vec![3, 3, 3], vec![3, 3, 3]]).unwrap();
		let i = GridSandpile::from_grid(GridType::Finite, vec![vec![2, 3, 2], vec![2, 3, 2]]).unwrap();
		assert_eq!(s.inverse(), i);
	}
}
