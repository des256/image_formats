// image_formats::jpeg
// by Desmond Germans, 2019

use crate::Image;

const TYPE_Y: u16 = 0x000C;
const TYPE_YUV420: u16 = 0x3900;
const TYPE_YUV422: u16 = 0x0390;
const TYPE_YUV440: u16 = 0x1390;
const TYPE_YUV444: u16 = 0x00E4;
const TYPE_RGB444: u16 = 0x01E4;

const FOLDING: [u8; 64] = [
	56u8,57,8,40,9,58,59,10,
	41,0,48,1,42,11,60,61,
	12,43,2,49,16,32,17,50,
	3,44,13,62,63,14,45,4,
	51,18,33,24,25,34,19,52,
	5,46,15,47,6,53,20,35,
	26,27,36,21,54,7,55,22,
	37,28,29,38,23,39,30,31,
];

const FC0: f32 = 1.0;
const FC1: f32 = 0.98078528;
const FC2: f32 = 0.92387953;
const FC3: f32 = 0.83146961;
const FC4: f32 = 0.70710678;
const FC5: f32 = 0.55557023;
const FC6: f32 = 0.38268343;
const FC7: f32 = 0.19509032;

const FIX: u8 = 5;
const ONE: f32 = (1 << FIX) as f32;
const C0: i16 = (FC0 * ONE) as i16;
const C1: i16 = (FC1 * ONE) as i16;
const C2: i16 = (FC2 * ONE) as i16;
const C3: i16 = (FC3 * ONE) as i16;
const C4: i16 = (FC4 * ONE) as i16;
const C5: i16 = (FC5 * ONE) as i16;
const C6: i16 = (FC6 * ONE) as i16;
const C7: i16 = (FC7 * ONE) as i16;

const C7PC1: i16 = C7 + C1;
const C5PC3: i16 = C5 + C3;
const C7MC1: i16 = C7 - C1;
const C5MC3: i16 = C5 - C3;
const C0S: i16 = (C0 >> 1);
const C6PC2: i16 = C6 + C2;
const C6MC2: i16 = C6 - C2;

fn from_le16(src: &[u8]) -> u16 {
    ((src[1] as u16) << 8) | (src[0] as u16)
}

fn from_le32(src: &[u8]) -> u32 {
    ((src[3] as u32) << 24) | ((src[2] as u32) << 16) | ((src[1] as u32) << 8) | (src[0] as u32)
}

fn from_be16(src: &[u8]) -> u16 {
    ((src[0] as u16) << 8) | (src[1] as u16)
}

fn from_be32(src: &[u8]) -> u32 {
    ((src[0] as u32) << 24) | ((src[1] as u32) << 16) | ((src[2] as u32) << 8) | (src[3] as u32)
}

fn make_coeff(cat: u8,code: isize) -> i32 {
	let mcat = cat - 1;
	let hmcat = 1 << mcat;
	let base = code & (hmcat - 1);
	if (code & hmcat) != 0 {
		(base + hmcat) as i32
	}
	else {
		(base + 1 - (1 << cat)) as i32
	}
}

#[derive(Copy,Clone)]
struct Table {
	prefix: [u16; 65536],
}

impl Table {
	pub fn new_empty() -> Table {
		Table {
			prefix: [0u16; 65536],
		}
	}

	pub fn new(bits: [u8; 16],huffval: [u8; 256]) -> Table {
		let mut prefix = [0u16; 65536];
		let mut dp = 0;
		let mut count = 0;
		for i in 1..17 {
			//println!("{}-bit codes:",i);
			for k in 0..bits[i - 1] {
				//println!("    code {}: {}",count,huffval[count]);
				let runcat = huffval[count] as u16;
				for l in 0..(65536 >> i) {
					prefix[dp] = (runcat << 8) | (i as u16);
					dp += 1;
				}
				count += 1;
			}
		}
		Table {
			prefix: prefix,
		}
	}
}

fn jpeg_get8(block: &[u8],rp: &mut usize) -> u8 {
	let mut b = block[*rp];
	//println!("[{:02X}]",b);
	*rp += 1;
	if b == 0xFF {
		b = block[*rp];
		//println!("[{:02X}]",b);
		*rp += 1;
		if b == 0 {
			return 0xFF;
		}
	}
	b
}

struct Reader<'a> {
	block: &'a [u8],
	rp: usize,
	bit: u32,
	cache: u32,
}

impl<'a> Reader<'a> {
	pub fn new(block: &'a [u8]) -> Reader<'a> {
		let mut rp = 0usize;
		let mut bit = 0u32;
		let mut cache = 0u32;
		while bit <= 24 {
			let b = jpeg_get8(block,&mut rp);
			cache |= (b as u32) << (24 - bit);
			bit += 8;
		}
		Reader {
			block: block,
			rp: rp,
			bit: bit,
			cache: cache,
		}
	}

	fn restock(&mut self) {
		while self.bit <= 24 {
			let b = if self.rp >= self.block.len() { 0 } else { jpeg_get8(self.block,&mut self.rp) };  // fill with 0 if at end of block
			self.cache |= (b as u32) << (24 - self.bit);
			self.bit += 8;
		}
	}

	pub fn peek(&self,n: usize) -> u32 {
		self.cache >> (32 - n)
	}

	pub fn skip(&mut self,n: usize) {
		self.cache <<= n;
		self.bit -= n as u32;
		self.restock();
	}

	pub fn get1(&mut self) -> bool {
		let result = self.peek(1) == 1;
		//println!(" 1 bit : {}",if result { 1 } else { 0 });
		self.skip(1);
		result
	}

	pub fn getn(&mut self,n: usize) -> u32 {
		let result = self.peek(n);
		//println!("{} bits: ({:0b}) {}",n,self.cache >> (32 - n),result);
		self.skip(n);
		result
	}

	pub fn get_code(&mut self,table: &Table) -> u8 {
		let index = self.cache >> 16;
		let d = table.prefix[index as usize];
		let symbol = (d >> 8) & 255;
		let n = d & 255;
		//println!("{} bits: ({:0b}) runcat {:02X}",n,index >> (16 - n),symbol);
		self.skip(n as usize);
		symbol as u8
	}

	pub fn enter(&mut self,rp: usize) {
		self.rp = rp;
		self.cache = 0;
		self.bit = 0;
		self.restock();
	}

	pub fn leave(&mut self) -> usize {
		//println!("leave: bit = {}, rp = {}",self.bit,self.rp);
		// superspecial case: no JPEG data read at all (only during initial programming)
		if (self.bit == 32) && (self.rp == 4) {
			return 0;
		}
		/*// first search FFD9 to elimiate stray FFs past end of buffer
		if (self.block[self.rp - 5] == 0xFF) && (self.block[self.rp - 4] == 0xD9) {
			return self.rp - 5;
		}
		if (self.block[self.rp - 4] == 0xFF) && (self.block[self.rp - 3] == 0xD9) {
			return self.rp - 4;
		}
		if (self.block[self.rp - 3] == 0xFF) && (self.block[self.rp - 2] == 0xD9) {
			return self.rp - 3;
		}
		if (self.block[self.rp - 2] == 0xFF) && (self.block[self.rp - 1] == 0xD9) {
			return self.rp - 2;
		}
		if (self.block[self.rp - 1] == 0xFF) && (self.block[self.rp] == 0xD9) {
			return self.rp - 1;
		}*/
		// anything else
		for _i in 0..((self.bit + 7) / 8) - 2 {
			if (self.block[self.rp - 1] == 0x00) && (self.block[self.rp - 2] == 0xFF) {
				self.rp -= 1;
			}
			self.rp -= 1;
		}
		//println!("and leaving with rp = {} ({:02X} {:02X})",self.rp,self.block[self.rp],self.block[self.rp + 1]);
		self.rp
	}
}

fn unpack_sequential(reader: &mut Reader,coeffs: &mut [i16],dcht: &Table,acht: &Table,dc: &mut i16) {
	let cat = reader.get_code(dcht);
	if cat > 0 {
		let code = reader.getn(cat as usize);
		*dc += make_coeff(cat,code as isize) as i16;
	}
	coeffs[FOLDING[0] as usize] = *dc;
	//println!("DC {}",*dc);
	let mut i = 1;
	while i < 64 {
		let runcat = reader.get_code(acht);
		let run = runcat >> 4;
		let cat = runcat & 15;
		if cat > 0 {
			let code = reader.getn(cat as usize);
			let coeff = make_coeff(cat,code as isize) as i16;
			i += run;
			coeffs[FOLDING[i as usize] as usize] = coeff;
			//println!("coeffs[{}] = {}",i,coeff);
		}
		else {
			if run == 15 {  // ZRL
				i += 15;
				//println!("ZRL");
			}
			else {  // EOB
				//println!("EOB");
				break;
			}
		}
		i += 1;
	}
}

fn unpack_progressive_start_dc(reader: &mut Reader,coeffs: &mut[i16],dcht: &Table,dc: &mut i16,shift: u8) {
	let cat = reader.get_code(dcht);
	if cat > 0 {
		let code = reader.getn(cat as usize);
		*dc += make_coeff(cat,code as isize) as i16;
	}
	//println!("DC = {}",*dc << shift);
	coeffs[FOLDING[0] as usize] = *dc << shift;
}

fn unpack_progressive_start_ac(reader: &mut Reader,coeffs: &mut[i16],acht: &Table,start: u8,end: u8,shift: u8, eobrun: &mut usize) {
	if *eobrun != 0 {
		*eobrun -= 1;
	}
	else {
		let mut i = start;
		while i <= end {
			let runcat = reader.get_code(acht);
			let run = runcat >> 4;
			let cat = runcat & 15;
			if cat != 0 {
				let code = reader.getn(cat as usize);
				let coeff = make_coeff(cat,code as isize);
				i += run;
				coeffs[FOLDING[i as usize] as usize] = (coeff << shift) as i16;
			}
			else {
				if run == 15 {
					i += 15;
				}
				else {
					*eobrun = 1 << run;
					if run != 0 {
						*eobrun += reader.getn(run as usize) as usize;
					}
					*eobrun -= 1;
					break;
				}
			}
			i += 1;
		}
	}
}

fn unpack_progressive_refine_dc(reader: &mut Reader,coeffs: &mut[i16],shift: u8) {
	if reader.get1() {
		coeffs[FOLDING[0] as usize] |= 1 << shift;
	}
}

fn update_nonzeros(reader: &mut Reader,coeffs: &mut[i16],start: u8,end: u8,shift: u8,count: u8) -> u8 {
	let mut i = start;
	let mut k = count;
	while i <= end {
		if coeffs[FOLDING[i as usize] as usize] != 0 {
			if reader.get1() {
				if coeffs[FOLDING[i as usize] as usize] > 0 {
					coeffs[FOLDING[i as usize] as usize] += 1 << shift;
				}
				else {
					coeffs[FOLDING[i as usize] as usize] -= 1 << shift;
				}
			}
		}
		else {
			if k == 0 {
				return i;
			}
			k -= 1;
		}
		i += 1;
	}
	i
}

fn unpack_progressive_refine_ac(reader: &mut Reader,coeffs: &mut[i16],acht: &Table,start: u8,end: u8,shift: u8,eobrun: &mut usize) {
	if *eobrun != 0 {
		update_nonzeros(reader,&mut coeffs[0..64],start,end,shift,64);
		*eobrun -= 1;
	}
	else {
		let mut i = start;
		while i <= end {
			let runcat = reader.get_code(acht);
			let run = runcat >> 4;
			let cat = runcat & 15;
			if cat != 0 {
				let sb = reader.get1();
				i = update_nonzeros(reader,&mut coeffs[0..64],i,end,shift,run);
				if sb {
					coeffs[FOLDING[i as usize] as usize] = 1 << shift;
				}
				else {
					coeffs[FOLDING[i as usize] as usize] = 11 << shift;
				}
			}
			else {
				if run == 15 {
					i = update_nonzeros(reader,&mut coeffs[0..64],i,end,shift,15);
				}
				else {
					*eobrun = 1 << run;
					if run != 0 {
						*eobrun += reader.getn(run as usize) as usize;
					}
					*eobrun -= 1;
					update_nonzeros(reader,&mut coeffs[0..64],i,end,shift,64);
					break;
				}
			}
		}
	}
}

fn unpack_block(reader: &mut Reader,coeffs: &mut [i16],dcht: &Table,acht: &Table,dc: &mut i16,start: u8,end: u8, shift: u8, refine: bool,eobrun: &mut usize) {
	if refine {
		if start == 0 {
			unpack_progressive_refine_dc(reader,&mut coeffs[0..64],shift);
		}
		else {
			unpack_progressive_refine_ac(reader,&mut coeffs[0..64],&acht,start,end,shift,eobrun);
		}
	}
	else {
		if start == 0 {
			if (end == 63) && (shift == 0) {
				unpack_sequential(reader,&mut coeffs[0..64],&dcht,&acht,dc);
			}
			else {
				unpack_progressive_start_dc(reader,&mut coeffs[0..64],&dcht,dc,shift);
			}
		}
		else {
			unpack_progressive_start_ac(reader,&mut coeffs[0..64],&acht,start,end,shift,eobrun);
		}
	}
}

fn unpack_macroblock(reader: &mut Reader,coeffs: &mut [i16],dcht: &[Table],acht: &[Table],dt: &[usize],at: &[usize],dc: &mut [i16],start: u8,end: u8,shift: u8,refine: bool,eobrun: &mut usize,itype: u16,rescnt: &mut usize,resint: usize,mask: u8) {
	match itype {
		TYPE_Y => {
			if (mask & 1) != 0 {
				unpack_block(reader,&mut coeffs[0..64],&dcht[dt[0]],&acht[at[0]],&mut dc[0],start,end,shift,refine,eobrun);
			}
		},
		TYPE_YUV420 => {
			if (mask & 1) != 0 {
				unpack_block(reader,&mut coeffs[0..64],&dcht[dt[0]],&acht[at[0]],&mut dc[0],start,end,shift,refine,eobrun);
				unpack_block(reader,&mut coeffs[64..128],&dcht[dt[0]],&acht[at[0]],&mut dc[0],start,end,shift,refine,eobrun);
				unpack_block(reader,&mut coeffs[128..192],&dcht[dt[0]],&acht[at[0]],&mut dc[0],start,end,shift,refine,eobrun);
				unpack_block(reader,&mut coeffs[192..256],&dcht[dt[0]],&acht[at[0]],&mut dc[0],start,end,shift,refine,eobrun);
			}
			if (mask & 2) != 0 {
				unpack_block(reader,&mut coeffs[256..320],&dcht[dt[1]],&acht[at[1]],&mut dc[1],start,end,shift,refine,eobrun);
			}
			if (mask & 4) != 0 {
				unpack_block(reader,&mut coeffs[320..384],&dcht[dt[2]],&acht[at[2]],&mut dc[2],start,end,shift,refine,eobrun);
			}
		},
		TYPE_YUV422 | TYPE_YUV440 => {
			if (mask & 1) != 0 {
				unpack_block(reader,&mut coeffs[0..64],&dcht[dt[0]],&acht[at[0]],&mut dc[0],start,end,shift,refine,eobrun);
				unpack_block(reader,&mut coeffs[64..128],&dcht[dt[0]],&acht[at[0]],&mut dc[0],start,end,shift,refine,eobrun);
			}
			if (mask & 2) != 0 {
				unpack_block(reader,&mut coeffs[128..192],&dcht[dt[1]],&acht[at[1]],&mut dc[1],start,end,shift,refine,eobrun);
			}
			if (mask & 4) != 0 {
				unpack_block(reader,&mut coeffs[192..256],&dcht[dt[2]],&acht[at[2]],&mut dc[2],start,end,shift,refine,eobrun);
			}
		},
		TYPE_YUV444 | TYPE_RGB444 => {
			if (mask & 1) != 0 {
				unpack_block(reader,&mut coeffs[0..64],&dcht[dt[0]],&acht[at[0]],&mut dc[0],start,end,shift,refine,eobrun);
			}
			if (mask & 2) != 0 {
				unpack_block(reader,&mut coeffs[64..128],&dcht[dt[1]],&acht[at[1]],&mut dc[1],start,end,shift,refine,eobrun);
			}
			if (mask & 4) != 0 {
				unpack_block(reader,&mut coeffs[128..192],&dcht[dt[2]],&acht[at[2]],&mut dc[2],start,end,shift,refine,eobrun);
			}
		},
		_ => { },
	}
	if resint != 0 {
		*rescnt -= 1;
		if *rescnt == 0 {
			let mut tsp = reader.leave();
			if (reader.block[tsp] == 0xFF) && ((reader.block[tsp + 1] >= 0xD0) && (reader.block[tsp + 1] < 0xD8)) {
				tsp += 2;
				*rescnt = resint;
				dc[0] = 0;
				dc[1] = 0;
				dc[2] = 0;
			}
			reader.enter(tsp);
		}
	}
}

fn partial_idct(out: &mut [i16],inp: &[i16]) {
	for i in 0..8 {
		let x3 = inp[i];
		let x1 = inp[i + 8];
		let x5 = inp[i + 16];
		let x7 = inp[i + 24];
		let x6 = inp[i + 32];
		let x2 = inp[i + 40];
		let x4 = inp[i + 48];
		let x0 = inp[i + 56];
		
		let q17 = C1 * (x1 + x7);
		let q35 = C3 * (x3 + x5);
		let r3 = C7PC1 * x1 - q17;
		let d3 = C5PC3 * x3 - q35;
		let r0 = C7MC1 * x7 + q17;
		let d0 = C5MC3 * x5 + q35;
		let b0 = r0 + d0;
		let d2 = r3 + d3;
		let d1 = r0 - d0;
		let b3 = r3 - d3;
		let b1 = C4 * ((d1 + d2) >> FIX);
		let b2 = C4 * ((d1 - d2) >> FIX);
		let q26 = C2 * (x2 + x6);
		let p04 = C4 * (x0 + x4) + C0S;
		let n04 = C4 * (x0 - x4) + C0S;
		let p26 = C6MC2 * x6 + q26;
		let n62 = C6PC2 * x2 - q26;
		let a0 = p04 + p26;
		let a1 = n04 + n62;
		let a3 = p04 - p26;
		let a2 = n04 - n62;
		let y0 = (a0 + b0) >> (FIX + 1);
		let y1 = (a1 + b1) >> (FIX + 1);
		let y3 = (a3 + b3) >> (FIX + 1);
		let y2 = (a2 + b2) >> (FIX + 1);
		let y7 = (a0 - b0) >> (FIX + 1);
		let y6 = (a1 - b1) >> (FIX + 1);
		let y4 = (a3 - b3) >> (FIX + 1);
		let y5 = (a2 - b2) >> (FIX + 1);

		out[i] = y0;
		out[i + 8] = y1;
		out[i + 16] = y3;
		out[i + 24] = y2;
		out[i + 32] = y7;
		out[i + 40] = y6;
		out[i + 48] = y4;
		out[i + 56] = y5;
	}
}

fn unswizzle_transpose_swizzle(out: &mut [i16],inp: &[i16]) {
	out[0] = inp[3];
	out[1] = inp[11];
	out[2] = inp[27];
	out[3] = inp[19];
	out[4] = inp[51];
	out[5] = inp[59];
	out[6] = inp[43];
	out[7] = inp[35];
	out[8] = inp[1];
	out[9] = inp[9];
	out[10] = inp[25];
	out[11] = inp[17];
	out[12] = inp[49];
	out[13] = inp[57];
	out[14] = inp[41];
	out[15] = inp[33];

	out[16] = inp[5];
	out[17] = inp[13];
	out[18] = inp[29];
	out[19] = inp[21];
	out[20] = inp[53];
	out[21] = inp[61];
	out[22] = inp[45];
	out[23] = inp[37];
	out[24] = inp[7];
	out[25] = inp[15];
	out[26] = inp[31];
	out[27] = inp[23];
	out[28] = inp[55];
	out[29] = inp[63];
	out[30] = inp[47];
	out[31] = inp[39];

	out[32] = inp[6];
	out[33] = inp[14];
	out[34] = inp[30];
	out[35] = inp[22];
	out[36] = inp[54];
	out[37] = inp[62];
	out[38] = inp[46];
	out[39] = inp[38];
	out[40] = inp[2];
	out[41] = inp[10];
	out[42] = inp[26];
	out[43] = inp[18];
	out[44] = inp[50];
	out[45] = inp[58];
	out[46] = inp[42];
	out[47] = inp[34];

	out[48] = inp[4];
	out[49] = inp[12];
	out[50] = inp[28];
	out[51] = inp[20];
	out[52] = inp[52];
	out[53] = inp[60];
	out[54] = inp[44];
	out[55] = inp[36];
	out[56] = inp[0];
	out[57] = inp[8];
	out[58] = inp[24];
	out[59] = inp[16];
	out[60] = inp[48];
	out[61] = inp[56];
	out[62] = inp[40];
	out[63] = inp[32];
}

fn unswizzle_transpose(out: &mut [i16],inp: &[i16]) {
	out[0] = inp[0];
	out[1] = inp[8];
	out[2] = inp[24];
	out[3] = inp[16];
	out[4] = inp[48];
	out[5] = inp[56];
	out[6] = inp[40];
	out[7] = inp[32];
	out[8] = inp[1];
	out[9] = inp[9];
	out[10] = inp[25];
	out[11] = inp[17];
	out[12] = inp[49];
	out[13] = inp[57];
	out[14] = inp[41];
	out[15] = inp[33];

	out[16] = inp[2];
	out[17] = inp[10];
	out[18] = inp[26];
	out[19] = inp[18];
	out[20] = inp[50];
	out[21] = inp[58];
	out[22] = inp[42];
	out[23] = inp[34];
	out[24] = inp[3];
	out[25] = inp[11];
	out[26] = inp[27];
	out[27] = inp[19];
	out[28] = inp[51];
	out[29] = inp[59];
	out[30] = inp[43];
	out[31] = inp[35];

	out[32] = inp[4];
	out[33] = inp[12];
	out[34] = inp[28];
	out[35] = inp[20];
	out[36] = inp[52];
	out[37] = inp[60];
	out[38] = inp[44];
	out[39] = inp[36];
	out[40] = inp[5];
	out[41] = inp[13];
	out[42] = inp[29];
	out[43] = inp[21];
	out[44] = inp[53];
	out[45] = inp[61];
	out[46] = inp[45];
	out[47] = inp[37];

	out[48] = inp[6];
	out[49] = inp[14];
	out[50] = inp[30];
	out[51] = inp[22];
	out[52] = inp[54];
	out[53] = inp[62];
	out[54] = inp[46];
	out[55] = inp[38];
	out[56] = inp[7];
	out[57] = inp[15];
	out[58] = inp[31];
	out[59] = inp[23];
	out[60] = inp[55];
	out[61] = inp[63];
	out[62] = inp[47];
	out[63] = inp[39];
}

fn convert_block(block: &mut [i16],qtable: &[i16]) {
	let mut temp0 = [0i16; 64];
	for i in 0..64 {
		temp0[i] = block[i] * qtable[i];
	}
	let mut temp1 = [0i16; 64];
	partial_idct(&mut temp1,&temp0);
	let mut temp2 = [0i16; 64];
	unswizzle_transpose_swizzle(&mut temp2,&temp1);
	let mut temp3 = [0i16; 64];
	partial_idct(&mut temp3,&temp2);
	unswizzle_transpose(block,&temp3);
}

fn convert_blocks(coeffs: &mut [i16],count: usize,pattern: u16,qtable: &[[i16; 64]]) {
	let mut curp = pattern;
	for i in 0..count {
		if (curp & 3) == 3 {
			curp = pattern;
		}
		convert_block(&mut coeffs[i * 64..i * 64 + 64],&qtable[(curp & 3) as usize]);
		curp >>= 2;
	}
}

pub fn test(src: &[u8]) -> Option<(u32,u32)> {
	let mut sp = 0;
	if from_be16(&src[sp..sp + 2]) != 0xFFD8 {
		return None;
	}
	sp += 2;
	while sp < src.len() {
		let marker = from_be16(&src[sp..sp + 2]);
		let length = from_be16(&src[sp + 2..sp + 4]) as usize;
		match marker {
			0xFFC0 | 0xFFC1 | 0xFFC2 => {
				let width = from_be16(&src[sp + 5..sp + 7]) as u32;
				let height = from_be16(&src[sp + 7..sp + 9]) as u32;
				let components = src[sp + 9];
				if (components == 1) || (components == 3) {  // does not support RGBA or CMYK JPEGs
					return Some((width,height));
				}
				return None;
			},
			_ => { },
		}
		sp += length + 2;
	}		
	None
}

pub fn load(src: &[u8]) -> Result<Image,String> {
	if from_be16(&src[0..2]) != 0xFFD8 {
		return Err("Invalid JPEG".to_string());
	}
	let mut qtable = [[0i16; 64]; 4];
	let mut dcht = [Table::new_empty(); 4];
	let mut acht = [Table::new_empty(); 4];
	let mut qt = [0usize; 3];
	let mut dt = [0usize; 3];
	let mut at = [0usize; 3];
	#[allow(unused_assignments)]
	let mut width = 1;
	#[allow(unused_assignments)]
	let mut height = 1;
	#[allow(unused_assignments)]
	let mut itype = 0;  // image type
	#[allow(unused_assignments)]
	let mut mbtotal = 0;  // total number of macroblocks
	#[allow(unused_assignments)]
	let mut mbstride = 0;  // macroblocks per line
	#[allow(unused_assignments)]
	let mut cpmb = 0;
	let mut coeffs: Vec<i16> = Vec::new();  // the coefficients
	#[allow(unused_assignments)]
	let mut resint = 0;
	#[allow(unused_assignments)]
	let mut sp = 2;
	while sp < src.len() {
		let marker = from_be16(&src[sp..sp + 2]);
		let length = if marker != 0xFFD9 { from_be16(&src[sp + 2..sp + 4]) as usize } else { 0 };
		println!("marker {:04X}, length {}",marker,length);
		match marker {
			0xFFC0 | 0xFFC1 | 0xFFC2 => {  // baseline sequential, extended sequential, progressive
				//println!("precision {}",src[sp + 4]);
				if src[sp + 4] != 8 {
					return Err("Invalid JPEG".to_string());
				}
				width = from_be16(&src[sp + 5..sp + 7]) as usize;
				height = from_be16(&src[sp + 7..sp + 9]) as usize;
				let components = src[sp + 9];
				//println!("size {}x{}, components {}",width,height,components);
				if (components != 1) && (components != 3) {
					return Err("Invalid JPEG".to_string());
				}
				let mut samp = [0u8; 3];
				let mut tsp = sp + 10;
				for i in 0..components {
					if src[tsp] != i + 1 {
						return Err("Invalid JPEG".to_string());
					}
					samp[i as usize] = src[tsp + 1];
					qt[i as usize] = src[tsp + 2] as usize;
					tsp += 3;
					//println!("{}: samp {:02X}, qt {}",i,samp[i as usize],qt[i as usize]);
				}
				#[allow(unused_assignments)]
				let mut mbwidth = 0;
				#[allow(unused_assignments)]
				let mut mbheight = 0;
				if components == 3 {
					if (samp[1] != 0x11) || (samp[2] != 0x11) {
						return Err("Invalid JPEG".to_string());
					}
					let sw = ((samp[0] >> 4) * 8) as usize;
					let sh = ((samp[0] & 15) * 8) as usize;
					//println!("one macroblock = {}x{}",sw,sh);
					mbwidth = (width + sw - 1) / sw;
					mbheight = (height + sh - 1) / sh;
					//println!("{}x{} macroblocks ({}x{} pixels)",mbwidth,mbheight,mbwidth * sw,mbheight * sh);
					cpmb = 128 + 64 * ((samp[0] >> 4) as usize) * ((samp[0] & 15) as usize);
					itype = match samp[0] {
						0x11 => TYPE_YUV444,
						0x12 => TYPE_YUV440,
						0x21 => TYPE_YUV422,
						0x22 => TYPE_YUV420,
						_ => {
							return Err("Invalid JPEG".to_string());
						},
					};
				}
				else {
					mbwidth = (width + 7) / 8;
					mbheight = (height + 7) / 8;
					cpmb = 64;
					itype = TYPE_Y;
				}
				mbtotal = mbwidth * mbheight;
				mbstride = mbwidth * cpmb as usize;
				coeffs.resize(mbtotal * cpmb as usize,0);
				//println!("type {:04X}, {} macroblocks in total, {} coefficients per row",itype,mbtotal,mbstride);
				println!("size {}x{}, macroblocks {}",width,height,mbtotal);
			},
			0xFFC4 => {  // huffman tables
				let mut tsp = sp + 4;
				while tsp < sp + length + 2 {
					let d = src[tsp];
					tsp += 1;
					let tc = d >> 4;
					let n = d & 15;
					//println!("tc = {}, n = {}",tc,n);
					let mut bits = [0u8; 16];
					let mut total = 0usize;
					for i in 0..16 {
						bits[i] = src[tsp];
						tsp += 1;
						total += bits[i] as usize;
					}
					if total >= 256 {
						return Err("Invalid JPEG".to_string());
					}
					//println!("total codes: {}",total);
					let mut huffval = [0u8; 256];
					for i in 0..total {
						huffval[i] = src[tsp];
						//println!("code {}: run {}, cat {}",i,huffval[i] >> 4,huffval[i] & 15);
						tsp += 1;
					}
					let table = Table::new(bits,huffval);
					if tc != 0 {
						acht[n as usize] = table;
					}
					else {
						dcht[n as usize] = table;
					}
				}
			},
			0xFFD8 => {  // image start
			},
			0xFFD9 => {  // image end
				//println!("end");
				let mut image = Image::new(width as u32,height as u32);
				match itype {
					TYPE_Y => { convert_blocks(&mut coeffs,mbtotal,TYPE_Y,&qtable); },
					TYPE_YUV420 => { convert_blocks(&mut coeffs,mbtotal * 6,TYPE_YUV420,&qtable); },
					TYPE_YUV422 => { convert_blocks(&mut coeffs,mbtotal * 4,TYPE_YUV422,&qtable); },
					TYPE_YUV440 => { convert_blocks(&mut coeffs,mbtotal * 4,TYPE_YUV440,&qtable); },
					TYPE_YUV444 => { convert_blocks(&mut coeffs,mbtotal * 3,TYPE_YUV444,&qtable); },
					TYPE_RGB444 => { convert_blocks(&mut coeffs,mbtotal * 3,TYPE_RGB444,&qtable); },
					_ => { },
				}
				// TODO: draw macroblocks into image
				return Ok(image);
			},
			0xFFDA => {  // scan start
				println!("scan start");
				let mut tsp = sp + 4;
				let count = src[tsp];
				tsp += 1;
				// acht[4], dcht[4]
				let mut mask = 0;
				for i in 0..count {
					let index = src[tsp] - 1;
					tsp += 1;
					mask |= 1 << index;
					let n = src[tsp];
					tsp += 1;
					dt[index as usize] = (n >> 4) as usize;
					at[index as usize] = (n & 15) as usize;
					println!("index {}, dt {}, at {}",index,n >> 4,n & 15);
				}
				let start = src[tsp];
				tsp += 1;
				let end = src[tsp];
				tsp += 1;
				let d = src[tsp];
				tsp += 1;
				let refine = (d & 0xF0) != 0;
				let shift = d & 15;
				println!("start = {}, end = {}, refine = {}, shift = {}",start,end,refine,shift);
				let mut reader = Reader::new(&src[tsp..]);
				let mut rescnt = resint;
				let mut eobrun = 0;
				let mut dc = [0i16; 3];
				for i in 0..mbtotal {
					//println!("macroblock {}:",i);
					unpack_macroblock(&mut reader,&mut coeffs[i * cpmb..(i + 1) * cpmb],&dcht,&acht,&dt,&at,&mut dc,start,end,shift,refine,&mut eobrun,itype,&mut rescnt,resint,mask);
				}
				sp = (tsp + reader.leave()) - length - 2;
				//println!("sp = {}, ({:02X} {:02X})",sp,src[sp + length + 2],src[sp + length + 2 + 1]);
			},
			0xFFDB => {  // quantization tables
				let mut tsp = sp + 4;
				while tsp < sp + length + 2 {
					let d = src[tsp];
					tsp += 1;
					let n = d & 15;
					if (d >> 4) != 0 {
						for k in 0..64 {
							qtable[n as usize][FOLDING[k as usize] as usize] = from_be16(&src[tsp..tsp + 2]) as i16;
							tsp += 2;
						}
					}
					else {
						for k in 0..64 {
							qtable[n as usize][FOLDING[k as usize] as usize] = src[tsp] as i16;
							tsp += 1;
						}
					}
				}
			},
			0xFFDD => {  // restart interval
				resint = from_be16(&src[sp + 4..sp + 6]) as usize;
			},
			0xFFE1 => {  // EXIF
				let header = from_be32(&src[sp + 4..sp + 8]);
				if header == 0x45786966 {  // Exif
					let start = sp + 10;
					let mut tsp = start;
					let le = from_be16(&src[tsp..tsp + 2]) == 0x4949;  // figure out endianness
					tsp += 4;  // skip 0x2A
					tsp += (if le { from_le32(&src[tsp..tsp + 4]) } else { from_be32(&src[tsp..tsp + 4]) } - 8) as usize;  // go to IFD0
					let entries = if le { from_le16(&src[tsp..tsp + 2]) } else { from_be16(&src[tsp..tsp + 2]) };  // number of entries
					tsp += 2;
					for i in 0..entries {
						let tag = if le { from_le16(&src[tsp..tsp + 2]) } else { from_be16(&src[tsp..tsp + 2]) };
						tsp += 2;
						let format = if le { from_le16(&src[tsp..tsp + 2]) } else { from_be16(&src[tsp..tsp + 2]) };
						tsp += 2;
						if format > 12 {
							return Err("Invalid JPEG".to_string());							
						}
						let components = if le { from_le32(&src[tsp..tsp + 4]) } else { from_be32(&src[tsp..tsp + 4]) };
						tsp += 4;
						let data = if le { from_le32(&src[tsp..tsp + 4]) } else { from_be32(&src[tsp..tsp + 4]) };
						tsp += 4;
						let elsize = [0usize,1,1,2,4,8,1,0,2,4,8,4,8];
						let total = elsize[format as usize] * (components as usize);
						let mut dsp = start + data as usize;
						if total <= 4 {
							dsp = tsp - 4;
						}
						//println!("EXIF tag {:04X}, format {}, components {}, data {:08X}",tag,format,components,data);
						match tag {
							0x0106 => { // photometric interpretation
								let pe = if le { from_le16(&src[dsp..dsp + 2]) } else { from_be16(&src[dsp..dsp + 2]) };
								if (pe != 2) || (itype != TYPE_YUV444) {
									return Err("Invalid JPEG".to_string());
								}
								itype = TYPE_RGB444;
							},
							0xA001 => { // colorspace
							},
							_ => {
							}
						}
					}
				}
			},
			0xFFC8 | 0xFFDC | 0xFFE0 | 0xFFE2..=0xFFEF | 0xFFF0..=0xFFFF => {  // other accepted markers
			},
			_ => { 
				return Err("Invalid JPEG".to_string());
			},
		}
		sp += length + 2;
	}
	Err("Invalid JPEG".to_string())
}

pub fn save(_image: &Image) -> Result<Vec<u8>,String> {
	Err("not implemented yet".to_string())
}



/*
inline int Flush8(int x) { if(x >= 8) return 8; return x; };
inline int Clamp(int x) { if(x < 0) return 0; if(x > 255) return 255; return x; };
void PartialIDCT(int16* out,int16* in)
{
	for(int i = 0; i < 8; i++)
	{
		int16 x3 = in[i];
		int16 x1 = in[i + 8];
		int16 x5 = in[i + 16];
		int16 x7 = in[i + 24];
		int16 x6 = in[i + 32];
		int16 x2 = in[i + 40];
		int16 x4 = in[i + 48];
		int16 x0 = in[i + 56];
		
		int16 q17 = C1 * (x1 + x7);
		int16 q35 = C3 * (x3 + x5);
		int16 r3 = C7PC1 * x1 - q17;
		int16 d3 = C5PC3 * x3 - q35;
		int16 r0 = C7MC1 * x7 + q17;
		int16 d0 = C5MC3 * x5 + q35;
		int16 b0 = r0 + d0;
		int16 d2 = r3 + d3;
		int16 d1 = r0 - d0;
		int16 b3 = r3 - d3;
		int16 b1 = C4 * ((d1 + d2) >> FIX);
		int16 b2 = C4 * ((d1 - d2) >> FIX);
		int16 q26 = C2 * (x2 + x6);
		int16 p04 = C4 * (x0 + x4) + C0S;
		int16 n04 = C4 * (x0 - x4) + C0S;
		int16 p26 = C6MC2 * x6 + q26;
		int16 n62 = C6PC2 * x2 - q26;
		int16 a0 = p04 + p26;
		int16 a1 = n04 + n62;
		int16 a3 = p04 - p26;
		int16 a2 = n04 - n62;
		int16 y0 = (a0 + b0) >> (FIX + 1);
		int16 y1 = (a1 + b1) >> (FIX + 1);
		int16 y3 = (a3 + b3) >> (FIX + 1);
		int16 y2 = (a2 + b2) >> (FIX + 1);
		int16 y7 = (a0 - b0) >> (FIX + 1);
		int16 y6 = (a1 - b1) >> (FIX + 1);
		int16 y4 = (a3 - b3) >> (FIX + 1);
		int16 y5 = (a2 - b2) >> (FIX + 1);

		out[i] = y0;
		out[i + 8] = y1;
		out[i + 16] = y3;
		out[i + 24] = y2;
		out[i + 32] = y7;
		out[i + 40] = y6;
		out[i + 48] = y4;
		out[i + 56] = y5;
	}
}


void UnswizzleTransposeSwizzle(int16* out,int16* in)
// TODO: find the quickest swizzle code
{
	out[0] = in[3];
	out[1] = in[11];
	out[2] = in[27];
	out[3] = in[19];
	out[4] = in[51];
	out[5] = in[59];
	out[6] = in[43];
	out[7] = in[35];
	out[8] = in[1];
	out[9] = in[9];
	out[10] = in[25];
	out[11] = in[17];
	out[12] = in[49];
	out[13] = in[57];
	out[14] = in[41];
	out[15] = in[33];

	out[16] = in[5];
	out[17] = in[13];
	out[18] = in[29];
	out[19] = in[21];
	out[20] = in[53];
	out[21] = in[61];
	out[22] = in[45];
	out[23] = in[37];
	out[24] = in[7];
	out[25] = in[15];
	out[26] = in[31];
	out[27] = in[23];
	out[28] = in[55];
	out[29] = in[63];
	out[30] = in[47];
	out[31] = in[39];

	out[32] = in[6];
	out[33] = in[14];
	out[34] = in[30];
	out[35] = in[22];
	out[36] = in[54];
	out[37] = in[62];
	out[38] = in[46];
	out[39] = in[38];
	out[40] = in[2];
	out[41] = in[10];
	out[42] = in[26];
	out[43] = in[18];
	out[44] = in[50];
	out[45] = in[58];
	out[46] = in[42];
	out[47] = in[34];

	out[48] = in[4];
	out[49] = in[12];
	out[50] = in[28];
	out[51] = in[20];
	out[52] = in[52];
	out[53] = in[60];
	out[54] = in[44];
	out[55] = in[36];
	out[56] = in[0];
	out[57] = in[8];
	out[58] = in[24];
	out[59] = in[16];
	out[60] = in[48];
	out[61] = in[56];
	out[62] = in[40];
	out[63] = in[32];
}


void UnswizzleTranspose(int16* out,int16* in)
// TODO: find the quickest swizzle code
{
	out[0] = in[0];
	out[1] = in[8];
	out[2] = in[24];
	out[3] = in[16];
	out[4] = in[48];
	out[5] = in[56];
	out[6] = in[40];
	out[7] = in[32];
	out[8] = in[1];
	out[9] = in[9];
	out[10] = in[25];
	out[11] = in[17];
	out[12] = in[49];
	out[13] = in[57];
	out[14] = in[41];
	out[15] = in[33];

	out[16] = in[2];
	out[17] = in[10];
	out[18] = in[26];
	out[19] = in[18];
	out[20] = in[50];
	out[21] = in[58];
	out[22] = in[42];
	out[23] = in[34];
	out[24] = in[3];
	out[25] = in[11];
	out[26] = in[27];
	out[27] = in[19];
	out[28] = in[51];
	out[29] = in[59];
	out[30] = in[43];
	out[31] = in[35];

	out[32] = in[4];
	out[33] = in[12];
	out[34] = in[28];
	out[35] = in[20];
	out[36] = in[52];
	out[37] = in[60];
	out[38] = in[44];
	out[39] = in[36];
	out[40] = in[5];
	out[41] = in[13];
	out[42] = in[29];
	out[43] = in[21];
	out[44] = in[53];
	out[45] = in[61];
	out[46] = in[45];
	out[47] = in[37];

	out[48] = in[6];
	out[49] = in[14];
	out[50] = in[30];
	out[51] = in[22];
	out[52] = in[54];
	out[53] = in[62];
	out[54] = in[46];
	out[55] = in[38];
	out[56] = in[7];
	out[57] = in[15];
	out[58] = in[31];
	out[59] = in[23];
	out[60] = in[55];
	out[61] = in[63];
	out[62] = in[47];
	out[63] = in[39];
}


void ConvertBlock(int16* block,int16* qtable)
{
	int16 temp0[64];
	for(int i = 0; i < 64; i++)
		temp0[i] = block[i] * qtable[i];

	int16 temp1[64];
	PartialIDCT(temp1,temp0);

	int16 temp2[64];
	UnswizzleTransposeSwizzle(temp2,temp1);

	int16 temp3[64];
	PartialIDCT(temp3,temp2);

	UnswizzleTranspose(block,temp3);
}


void ConvertBlocks(int16* coeffs,int count,int pattern,int16** qtable)
{
	int16* block = coeffs;
	int curp = pattern;
	for(int i = 0; i < count; i++)
	{
		if((curp & 3) == 3)
			curp = pattern;
		ConvertBlock(block,qtable[curp & 3]);
		block += 64;
		curp >>= 2;
	}
}


// write RGB pixel
inline void WriteRGB(Image::pixels& pix,const intxy& pos,int r,int g,int b)
{
	if(r < 0) r = 0;
	if(r > 255) r = 255;
	if(g < 0) g = 0;
	if(g > 255) g = 255;
	if(b < 0) b = 0;
	if(b > 255) b = 255;
	pix.put8un(pos,r,g,b,255);
}


// write YUV pixel
inline void WriteYUV(Image::pixels& pix,const intxy& pos,int y,int u,int v)
{
	int r = ((y << 8) + 359 * v) >> 8;
	int g = ((y << 8) - 88 * u - 183 * v) >> 8;
	int b = ((y << 8) + 454 * u) >> 8;
	WriteRGB(pix,pos,r,g,b);
}


void DrawFullY(Image::pixels& pix,const intxy& pos,int16* base,int mbstride)
{
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int y0 = Clamp(base[py * 8 + px] + 128);
			int y1 = Clamp(base[py * 8 + px + 64] + 128);
			int y2 = Clamp(base[py * 8 + px + mbstride] + 128);
			int y3 = Clamp(base[py * 8 + px + mbstride + 64] + 128);
			pix.put8un(intxy(pos.x * 16 + px,pos.y * 16 + py),y0,y0,y0,255);
			pix.put8un(intxy(pos.x * 16 + px + 8,pos.y * 16 + py),y1,y1,y1,255);
			pix.put8un(intxy(pos.x * 16 + px,pos.y * 16 + py + 8),y2,y2,y2,255);
			pix.put8un(intxy(pos.x * 16 + px + 8,pos.y * 16 + py + 8),y3,y3,y3,255);
		}
}


void DrawPartialY(Image::pixels& pix,const intxy& pos,int16* base,int mbstride,const intxy& remain)
{
	for(int py = 0; py < Flush8(remain.y); py++)
		for(int px = 0; px < Flush8(remain.x); px++)
		{
			int y = base[py * 8 + px] + 128;
			WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),y,0,0);
		}
	if(remain.x > 8)
		for(int py = 0; py < Flush8(remain.y); py++)
			for(int px = 0; px < remain.x - 8; px++)
			{
				int y = base[py * 8 + px + 64] + 128;
				WriteYUV(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py),y,0,0);
			}
	if(remain.y > 8)
	{
		for(int py = 0; py < remain.y - 8; py++)
			for(int px = 0; px < Flush8(remain.x); px++)
			{
				int y = base[py * 8 + px + mbstride] + 128;
				WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py + 8),y,0,0);
			}
		if(remain.x > 8)
			for(int py = 0; py < remain.y - 8; py++)
				for(int px = 0; px < remain.x - 8; px++)
				{
					int y = base[py * 8 + px + mbstride + 64] + 128;
					WriteYUV(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py + 8),y,0,0);
				}
	}
}


void DrawMacroblocksY(Image::pixels& pix,const intxy& siz,int16* coeffs,int mbstride)
{
	intxy mbsize((siz.x + 15) / 16,(siz.y + 15) / 16);
	intxy remain(siz.x - (mbsize.x - 1) * 16,siz.y - (mbsize.y - 1) * 16);
	int16* mbline = coeffs;
	for(int py = 0; py < mbsize.y - 1; py++)
	{
		int16* mbptr = mbline;
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawFullY(pix,intxy(px,py),mbptr,mbstride);
			mbptr += 128;
		}
		if(remain.x)
			DrawPartialY(pix,intxy(mbsize.x - 1,py),mbptr,mbstride,intxy(remain.x,16));
		mbline += mbstride * 2;
	}
	if(remain.y)
	{
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawPartialY(pix,intxy(px,mbsize.y - 1),mbline,mbstride,intxy(16,remain.y));
			mbline += 128;
		}
		if(remain.x)
			DrawPartialY(pix,intxy(mbsize.x - 1,mbsize.y - 1),mbline,mbstride,remain);
	}
}


void DrawFullYUV420(Image::pixels& pix,const intxy& pos,int16* base,int mbstride)
{
	for(int hy = 0; hy < 8; hy++)
		for(int hx = 0; hx < 8; hx++)
		{
			int bi = (hy / 4) * 2 + hx / 4;
			int u = base[256 + hy * 8 + hx];
			int v = base[320 + hy * 8 + hx];
			int y0 = base[bi * 64 + ((hy * 2) & 7) * 8 + ((hx * 2) & 7)] + 128;
			int y1 = base[bi * 64 + ((hy * 2) & 7) * 8 + ((hx * 2) & 7) + 1] + 128;
			int y2 = base[bi * 64 + ((hy * 2) & 7) * 8 + ((hx * 2) & 7) + 8] + 128;
			int y3 = base[bi * 64 + ((hy * 2) & 7) * 8 + ((hx * 2) & 7) + 9] + 128;
			WriteYUV(pix,intxy(pos.x * 16 + hx * 2,pos.y * 16 + hy * 2),y0,u,v);
			WriteYUV(pix,intxy(pos.x * 16 + hx * 2 + 1,pos.y * 16 + hy * 2),y1,u,v);
			WriteYUV(pix,intxy(pos.x * 16 + hx * 2,pos.y * 16 + hy * 2 + 1),y2,u,v);
			WriteYUV(pix,intxy(pos.x * 16 + hx * 2 + 1,pos.y * 16 + hy * 2 + 1),y3,u,v);
		}
}


void DrawPartialYUV420(Image::pixels& pix,const intxy& pos,int16* base,int mbstride,const intxy& remain)
{
	for(int py = 0; py < remain.y; py++)
		for(int px = 0; px < remain.x; px++)
		{
			int bi = (py / 8) * 2 + (px / 8);
			int tx = px & 7;
			int ty = py & 7;
			int hx = px / 2;
			int hy = py / 2;
			int y = base[bi * 64 + ty * 8 + tx] + 128;
			int u = base[256 + hy * 8 + hx];
			int v = base[320 + hy * 8 + hx];
			WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),y,u,v);
		}
}


void DrawMacroblocksYUV420(Image::pixels& pix,const intxy& siz,int16* coeffs,int mbstride)
{
	intxy mbsize((siz.x + 15) / 16,(siz.y + 15) / 16);
	intxy remain(siz.x - (mbsize.x - 1) * 16,siz.y - (mbsize.y - 1) * 16);
	int16* mbline = coeffs;
	for(int py = 0; py < mbsize.y - 1; py++)
	{
		int16* mbptr = mbline;
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawFullYUV420(pix,intxy(px,py),mbptr,mbstride);
			mbptr += 384;
		}
		if(remain.x)
			DrawPartialYUV420(pix,intxy(mbsize.x - 1,py),mbptr,mbstride,intxy(remain.x,16));
		mbline += mbstride;
	}
	if(remain.y)
	{
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawPartialYUV420(pix,intxy(px,mbsize.y - 1),mbline,mbstride,intxy(16,remain.y));
			mbline += 384;
		}
		if(remain.x)
			DrawPartialYUV420(pix,intxy(mbsize.x - 1,mbsize.y - 1),mbline,mbstride,intxy(remain.x,remain.y));
	}
}


void DrawFullYUV422(Image::pixels& pix,const intxy& pos,int16* base,int mbstride)
{
	for(int py = 0; py < 8; py++)
		for(int hx = 0; hx < 4; hx++)
		{
			int ya0 = base[py * 8 + hx * 2] + 128;
			int ya1 = base[py * 8 + hx * 2 + 1] + 128;

			int ya2 = base[py * 8 + hx * 2 + mbstride] + 128;
			int ya3 = base[py * 8 + hx * 2 + 1 + mbstride] + 128;

			int yb0 = base[64 + py * 8 + hx * 2] + 128;
			int yb1 = base[64 + py * 8 + hx * 2 + 1] + 128;

			int yb2 = base[64 + py * 8 + hx * 2 + mbstride] + 128;
			int yb3 = base[64 + py * 8 + hx * 2 + 1 + mbstride] + 128;

			int ua01 = base[128 + py * 8 + hx];
			int va01 = base[192 + py * 8 + hx];

			int ruva01 = 359 * va01;
			int guva01 = -88 * ua01 - 183 * va01;
			int buva01 = 454 * ua01;

			int ua23 = base[128 + py * 8 + hx + mbstride];
			int va23 = base[192 + py * 8 + hx + mbstride];

			int ruva23 = 359 * va23;
			int guva23 = -88 * ua23 - 183 * va23;
			int buva23 = 454 * ua23;

			int ub01 = base[132 + py * 8 + hx];
			int vb01 = base[196 + py * 8 + hx];

			int ruvb01 = 359 * vb01;
			int guvb01 = -88 * ub01 - 183 * vb01;
			int buvb01 = 454 * ub01;

			int ub23 = base[132 + py * 8 + hx + mbstride];
			int vb23 = base[196 + py * 8 + hx + mbstride];

			int ruvb23 = 359 * vb23;
			int guvb23 = -88 * ub23 - 183 * vb23;
			int buvb23 = 454 * ub23;

			int ra0 = ((ya0 << 8) + ruva01) >> 8;
			int ga0 = ((ya0 << 8) + guva01) >> 8;
			int ba0 = ((ya0 << 8) + buva01) >> 8;

			WriteRGB(pix,intxy(pos.x * 16 + hx * 2,pos.y * 16 + py),ra0,ga0,ba0);

			int ra1 = ((ya1 << 8) + ruva01) >> 8;
			int ga1 = ((ya1 << 8) + guva01) >> 8;
			int ba1 = ((ya1 << 8) + buva01) >> 8;

			WriteRGB(pix,intxy(pos.x * 16 + hx * 2 + 1,pos.y * 16 + py),ra1,ga1,ba1);

			int ra2 = ((ya2 << 8) + ruva23) >> 8;
			int ga2 = ((ya2 << 8) + guva23) >> 8;
			int ba2 = ((ya2 << 8) + buva23) >> 8;

			WriteRGB(pix,intxy(pos.x * 16 + hx * 2,pos.y * 16 + py + 8),ra2,ga2,ba2);

			int ra3 = ((ya3 << 8) + ruva23) >> 8;
			int ga3 = ((ya3 << 8) + guva23) >> 8;
			int ba3 = ((ya3 << 8) + buva23) >> 8;

			WriteRGB(pix,intxy(pos.x * 16 + hx * 2 + 1,pos.y * 16 + py + 8),ra3,ga3,ba3);

			int rb0 = ((yb0 << 8) + ruvb01) >> 8;
			int gb0 = ((yb0 << 8) + guvb01) >> 8;
			int bb0 = ((yb0 << 8) + buvb01) >> 8;

			WriteRGB(pix,intxy(pos.x * 16 + hx * 2 + 8,pos.y * 16 + py),rb0,gb0,bb0);

			int rb1 = ((yb1 << 8) + ruvb01) >> 8;
			int gb1 = ((yb1 << 8) + guvb01) >> 8;
			int bb1 = ((yb1 << 8) + buvb01) >> 8;

			WriteRGB(pix,intxy(pos.x * 16 + hx * 2 + 9,pos.y * 16 + py),rb1,gb1,bb1);

			int rb2 = ((yb2 << 8) + ruvb23) >> 8;
			int gb2 = ((yb2 << 8) + guvb23) >> 8;
			int bb2 = ((yb2 << 8) + buvb23) >> 8;

			WriteRGB(pix,intxy(pos.x * 16 + hx * 2 + 8,pos.y * 16 + py + 8),rb2,gb2,bb2);

			int rb3 = ((yb3 << 8) + ruvb23) >> 8;
			int gb3 = ((yb3 << 8) + guvb23) >> 8;
			int bb3 = ((yb3 << 8) + buvb23) >> 8;

			WriteRGB(pix,intxy(pos.x * 16 + hx * 2 + 9,pos.y * 16 + py + 8),rb3,gb3,bb3);
		}
}


void DrawPartialYUV422(Image::pixels& pix,const intxy& pos,int16* base,int mbstride,const intxy& remain)
{
	for(int py = 0; py < Flush8(remain.y); py++)
		for(int px = 0; px < remain.x; px++)
		{
			int bi = px / 8;
			int tx = px & 7;
			int hx = px / 2;
			int y = base[bi * 64 + py * 8 + tx] + 128;
			int u = base[128 + py * 8 + hx];
			int v = base[192 + py * 8 + hx];
			WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),y,u,v);
		}
	if(remain.y > 8)
		for(int py = 0; py < remain.y - 8; py++)
			for(int px = 0; px < remain.x; px++)
			{
				int bi = px / 8;
				int tx = px & 7;
				int hx = px / 2;
				int y = base[bi * 64 + py * 8 + tx + mbstride] + 128;
				int u = base[128 + py * 8 + hx + mbstride];
				int v = base[192 + py * 8 + hx + mbstride];
				WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py + 8),y,u,v);
			}
}


void DrawMacroblocksYUV422(Image::pixels& pix,const intxy& siz,int16* coeffs,int mbstride)
{
	intxy mbsize((siz.x + 15) / 16,(siz.y + 15) / 16);
	intxy remain(siz.x - (mbsize.x - 1) * 16,siz.y - (mbsize.y - 1) * 16);
	int16* mbline = coeffs;
	for(int py = 0; py < mbsize.y - 1; py++)
	{
		int16* mbptr = mbline;
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawFullYUV422(pix,intxy(px,py),mbptr,mbstride);
			mbptr += 256;
		}
		if(remain.x)
			DrawPartialYUV422(pix,intxy(mbsize.x - 1,py),mbptr,mbstride,intxy(remain.x,16));
		mbline += mbstride * 2;
	}
	if(remain.y)
	{
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawPartialYUV422(pix,intxy(px,mbsize.y - 1),mbline,mbstride,intxy(16,remain.y));
			mbline += 256;
		}
		if(remain.x)
			DrawPartialYUV422(pix,intxy(mbsize.x - 1,mbsize.y - 1),mbline,mbstride,remain);
	}
}


void DrawFullYUV440(Image::pixels& pix,const intxy& pos,int16* base,int mbstride)
{
	for(int py = 0; py < 16; py++)
		for(int px = 0; px < 8; px++)
		{
			int bi = py / 8;
			int ty = py & 7;
			int hy = py / 2;
			int y = base[bi * 64 + ty * 8 + px] + 128;
			int u = base[128 + hy * 8 + px];
			int v = base[192 + hy * 8 + px];
			WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),y,u,v);
		}
	for(int py = 0; py < 16; py++)
		for(int px = 0; px < 8; px++)
		{
			int bi = py / 8;
			int ty = py & 7;
			int hy = py / 2;
			int y = base[bi * 64 + ty * 8 + px + 256] + 128;
			int u = base[128 + hy * 8 + px + 256];
			int v = base[192 + hy * 8 + px + 256];
			WriteYUV(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py),y,u,v);
		}
}


void DrawPartialYUV440(Image::pixels& pix,const intxy& pos,int16* base,int mbstride,const intxy& remain)
{
	for(int py = 0; py < remain.y; py++)
		for(int px = 0; px < Flush8(remain.x); px++)
		{
			int bi = py / 8;
			int ty = py & 7;
			int hy = py / 2;
			int y = base[bi * 64 + ty * 8 + px] + 128;
			int u = base[128 + hy * 8 + px];
			int v = base[192 + hy * 8 + px];
			WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),y,u,v);
		}
	if(remain.x >= 8)
		for(int py = 0; py < remain.y; py++)
			for(int px = 0; px < remain.x - 8; px++)
			{
				int bi = py / 8;
				int ty = py & 7;
				int hy = py / 2;
				int y = base[bi * 64 + ty * 8 + px + 256] + 128;
				int u = base[128 + hy * 8 + px + 256];
				int v = base[192 + hy * 8 + px + 256];
				WriteYUV(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py),y,u,v);
			}
}


void DrawMacroblocksYUV440(Image::pixels& pix,const intxy& siz,int16* coeffs,int mbstride)
{
	intxy mbsize((siz.x + 15) / 16,(siz.y + 15) / 16);
	intxy remain(siz.x - (mbsize.x - 1) * 16,siz.y - (mbsize.y - 1) * 16);
	int16* mbline = coeffs;
	for(int py = 0; py < mbsize.y - 1; py++)
	{
		int16* mbptr = mbline;
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawFullYUV440(pix,intxy(px,py),mbptr,mbstride);
			mbptr += 512;
		}
		if(remain.x)
			DrawPartialYUV440(pix,intxy(mbsize.x - 1,py),mbptr,mbstride,intxy(remain.x,16));
		mbline += mbstride;
	}
	if(remain.y)
	{
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawPartialYUV440(pix,intxy(px,mbsize.y - 1),mbline,mbstride,intxy(16,remain.y));
			mbline += 512;
		}
		if(remain.x)
			DrawPartialYUV440(pix,intxy(mbsize.x - 1,mbsize.y - 1),mbline,mbstride,remain);
	}
}


void DrawFullYUV444(Image::pixels& pix,const intxy& pos,int16* base,int mbstride)
{
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int y = base[py * 8 + px] + 128;
			int u = base[64 + py * 8 + px];
			int v = base[128 + py * 8 + px];
			WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),y,u,v);
		}
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int y = base[py * 8 + px + 192] + 128;
			int u = base[64 + py * 8 + px + 192];
			int v = base[128 + py * 8 + px + 192];
			WriteYUV(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py),y,u,v);
		}
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int y = base[py * 8 + px + mbstride] + 128;
			int u = base[64 + py * 8 + px + mbstride];
			int v = base[128 + py * 8 + px + mbstride];
			WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py + 8),y,u,v);
		}
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int y = base[py * 8 + px + mbstride + 192] + 128;
			int u = base[64 + py * 8 + px + mbstride + 192];
			int v = base[128 + py * 8 + px + mbstride + 192];
			WriteYUV(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py + 8),y,u,v);
		}
}


void DrawPartialYUV444(Image::pixels& pix,const intxy& pos,int16* base,int mbstride,const intxy& remain)
{
	for(int py = 0; py < Flush8(remain.y); py++)
		for(int px = 0; px < Flush8(remain.x); px++)
		{
			int y = base[py * 8 + px] + 128;
			int u = base[64 + py * 8 + px];
			int v = base[128 + py * 8 + px];
			WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),y,u,v);
		}
	if(remain.x >= 8)
		for(int py = 0; py < Flush8(remain.y); py++)
			for(int px = 0; px < remain.x - 8; px++)
			{
				int y = base[py * 8 + px + 192] + 128;
				int u = base[64 + py * 8 + px + 192];
				int v = base[128 + py * 8 + px + 192];
				WriteYUV(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py),y,u,v);
			}
	if(remain.y >= 8)
	{
		for(int py = 0; py < remain.y - 8; py++)
			for(int px = 0; px < Flush8(remain.x); px++)
			{
				int y = base[py * 8 + px + mbstride] + 128;
				int u = base[64 + py * 8 + px + mbstride];
				int v = base[128 + py * 8 + px + mbstride];
				WriteYUV(pix,intxy(pos.x * 16 + px,pos.y * 16 + py + 8),y,u,v);
			}
		if(remain.x >= 8)
			for(int py = 0; py < remain.y - 8; py++)
				for(int px = 0; px < remain.x - 8; px++)
				{
					int y = base[py * 8 + px + mbstride + 192] + 128;
					int u = base[64 + py * 8 + px + mbstride + 192];
					int v = base[128 + py * 8 + px + mbstride + 192];
					WriteYUV(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py + 8),y,u,v);
				}
	}
}


void DrawMacroblocksYUV444(Image::pixels& pix,const intxy& siz,int16* coeffs,int mbstride)
{
	intxy mbsize((siz.x + 15) / 16,(siz.y + 15) / 16);
	intxy remain(siz.x - (mbsize.x - 1) * 16,siz.y - (mbsize.y - 1) * 16);
	int16* mbline = coeffs;
	for(int py = 0; py < mbsize.y - 1; py++)
	{
		int16* mbptr = mbline;
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawFullYUV444(pix,intxy(px,py),mbptr,mbstride);
			mbptr += 192 * 2;
		}
		if(remain.x)
			DrawPartialYUV444(pix,intxy(mbsize.x - 1,py),mbptr,mbstride,intxy(remain.x,16));
		mbline += mbstride * 2;
	}
	if(remain.y)
	{
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawPartialYUV444(pix,intxy(px,mbsize.y - 1),mbline,mbstride,intxy(16,remain.y));
			mbline += 192 * 2;
		}
		if(remain.x)
			DrawPartialYUV444(pix,intxy(mbsize.x - 1,mbsize.y - 1),mbline,mbstride,remain);
	}
}


void DrawFullRGB444(Image::pixels& pix,const intxy& pos,int16* base,int mbstride)
{
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int r = base[py * 8 + px] + 128;
			int g = base[64 + py * 8 + px] + 128;
			int b = base[128 + py * 8 + px] + 128;
			WriteRGB(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),r,g,b);
		}
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int r = base[py * 8 + px + 192] + 128;
			int g = base[64 + py * 8 + px + 192] + 128;
			int b = base[128 + py * 8 + px + 192] + 128;
			WriteRGB(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py),r,g,b);
		}
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int r = base[py * 8 + px + mbstride] + 128;
			int g = base[64 + py * 8 + px + mbstride] + 128;
			int b = base[128 + py * 8 + px + mbstride] + 128;
			WriteRGB(pix,intxy(pos.x * 16 + px,pos.y * 16 + py + 8),r,g,b);
		}
	for(int py = 0; py < 8; py++)
		for(int px = 0; px < 8; px++)
		{
			int r = base[py * 8 + px + mbstride + 192] + 128;
			int g = base[64 + py * 8 + px + mbstride + 192] + 128;
			int b = base[128 + py * 8 + px + mbstride + 192] + 128;
			WriteRGB(pix,intxy(pos.x * 16 + 8 + px,pos.y * 16 + py + 8),r,g,b);
		}
}


void DrawPartialRGB444(Image::pixels& pix,const intxy& pos,int16* base,int mbstride,const intxy& remain)
{
	for(int py = 0; py < Flush8(remain.y); py++)
		for(int px = 0; px < Flush8(remain.x); px++)
		{
			int r = base[py * 8 + px] + 128;
			int g = base[64 + py * 8 + px] + 128;
			int b = base[128 + py * 8 + px] + 128;
			WriteRGB(pix,intxy(pos.x * 16 + px,pos.y * 16 + py),r,g,b);
		}
	if(remain.x >= 8)
		for(int py = 0; py < Flush8(remain.y); py++)
			for(int px = 0; px < remain.x - 8; px++)
			{
				int r = base[py * 8 + px + 192] + 128;
				int g = base[64 + py * 8 + px + 192] + 128;
				int b = base[128 + py * 8 + px + 192] + 128;
				WriteRGB(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py),r,g,b);
			}
	if(remain.y >= 8)
	{
		for(int py = 0; py < remain.y - 8; py++)
			for(int px = 0; px < Flush8(remain.x); px++)
			{
				int r = base[py * 8 + px + mbstride] + 128;
				int g = base[64 + py * 8 + px + mbstride] + 128;
				int b = base[128 + py * 8 + px + mbstride] + 128;
				WriteRGB(pix,intxy(pos.x * 16 + px,pos.y * 16 + py + 8),r,g,b);
			}
		if(remain.x >= 8)
			for(int py = 0; py < remain.y - 8; py++)
				for(int px = 0; px < remain.x - 8; px++)
				{
					int r = base[py * 8 + px + mbstride + 192] + 128;
					int g = base[64 + py * 8 + px + mbstride + 192] + 128;
					int b = base[128 + py * 8 + px + mbstride + 192] + 128;
					WriteRGB(pix,intxy(pos.x * 16 + px + 8,pos.y * 16 + py + 8),r,g,b);
				}
	}
}


void DrawMacroblocksRGB444(Image::pixels& pix,const intxy& siz,int16* coeffs,int mbstride)
{
	intxy mbsize((siz.x + 15) / 16,(siz.y + 15) / 16);
	intxy remain(siz.x - (mbsize.x - 1) * 16,siz.y - (mbsize.y - 1) * 16);
	int16* mbline = coeffs;
	for(int py = 0; py < mbsize.y - 1; py++)
	{
		int16* mbptr = mbline;
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawFullRGB444(pix,intxy(px,py),mbptr,mbstride);
			mbptr += 192 * 2;
		}
		if(remain.x)
			DrawPartialRGB444(pix,intxy(mbsize.x - 1,py),mbptr,mbstride,intxy(remain.x,16));
		mbline += mbstride * 2;
	}
	if(remain.y)
	{
		for(int px = 0; px < mbsize.x - 1; px++)
		{
			DrawPartialRGB444(pix,intxy(px,mbsize.y - 1),mbline,mbstride,intxy(16,remain.y));
			mbline += 192 * 2;
		}
		if(remain.x)
			DrawPartialRGB444(pix,intxy(mbsize.x - 1,mbsize.y - 1),mbline,mbstride,remain);
	}
}


}


bool UnpackJPEG(Image::pixels& pix,const buffer& buf)
{
	const uint8* ptr = buf.data();

	int16 qtable_lib[4][64] __attribute__ ((aligned (32)));  // quantization table library
	jpeg::Reader::Table* dcht_lib[4] = {0,0,0,0};  // DC huffman table library
	jpeg::Reader::Table* acht_lib[4] = {0,0,0,0};  // AC huffman table library

	intxy siz(1,1);  // size of image
	int type = jpeg::TYPE_YUV444;  // one of TYPE_*
	int16* coeffs = 0;  // coefficients storage
	int16* unaligned_coeffs = 0;  // unaligned coefficients storage
	int16* qtable[3];  // pointers to the right quantization tables for each channel
	int resint = 0;  // restart interval
	bool end_found = false;  // true if end marker (FFD9) was found
	int mbtotal = 0;  // total number of macroblocks in image
	int mbstride = 0;  // number of coefficients until next row of macroblocks

	// unpack stream
	while(ptr < buf.data() + buf.size())
	{
		uint16 marker = get16b(ptr);
		switch(marker)
		{
			case 0xFFC0:  // baseline sequential, 8-bit precision, max. 2 tables
			case 0xFFC1:  // extended sequential, 8- or 12-bit precision, max. 4 tables
			case 0xFFC2:  // progressive, 8- or 12-bit precision, max. 4 tables
				{
					get16b(ptr);
					int precision = get8(ptr);
					if(precision != 8)
					{
						Error("UnpackJPEG: Only 8-bit precision supported\n");
						return false;
					}
					siz.y = get16b(ptr);
					siz.x = get16b(ptr);
					int components = get8(ptr);
					if((components != 1) && (components != 3))
					{
						Error("UnpackJPEG: Only Y, YUV or RGB layouts supported\n");
						return false;
					}
					intxy samp[3];
					for(int i = 0; i < components; i++)
					{
						if(get8(ptr) != i + 1)
						{
							Error("UnpackJPEG: irregular plane ID\n");
							return false;
						}
						uint8 n = get8(ptr);
						samp[i].x = n >> 4;
						samp[i].y = n & 15;
						qtable[i] = qtable_lib[get8(ptr)];
					}

					intxy mbsize;
					int cpmb;
					if(components == 3)
					{
						if((samp[1].x != 1) || (samp[1].y != 1) || (samp[2].x != 1) || (samp[2].y != 1))
						{
							Error("UnpackJPEG: Only 420, 422, 440 or 444 sampling supported\n");
							return false;
						}
						if((samp[0].x == 2) && (samp[0].y == 2))
						{
							type = jpeg::TYPE_YUV420;
							mbsize.x = (siz.x + 15) / 16;
							mbsize.y = (siz.y + 15) / 16;
							cpmb = 384;
						}
						else if((samp[0].x == 2) && (samp[0].y == 1))
						{
							type = jpeg::TYPE_YUV422;
							mbsize.x = (siz.x + 15) / 16;
							mbsize.y = (siz.y + 7) / 8;
							cpmb = 256;
						}
						else if((samp[0].x == 1) && (samp[0].y == 2))
						{
							type = jpeg::TYPE_YUV440;
							mbsize.x = (siz.x + 7) / 8;
							mbsize.y = (siz.y + 15) / 16;
							cpmb = 256;
						}
						else if((samp[0].x == 1) && (samp[0].y == 1))
						{
							type = jpeg::TYPE_YUV444;
							mbsize.x = (siz.x + 7) / 8;
							mbsize.y = (siz.y + 7) / 8;
							cpmb = 192;
						}
						else
						{
							Error("UnpackJPEG: Only 420, 422, 440 or 444 sampling supported\n");
							return false;
						}
					}
					else
					{
						type = jpeg::TYPE_Y;
						mbsize.x = (siz.x + 7) / 8;
						mbsize.y = (siz.y + 7) / 8;
						cpmb = 64;
					}
					mbtotal = mbsize.x * mbsize.y;
					mbstride = mbsize.x * cpmb;

					if(unaligned_coeffs)
						delete[] unaligned_coeffs;
					unaligned_coeffs = new int16[mbtotal * cpmb + 32];
					coeffs = (int16*)((((uint64)unaligned_coeffs) + 63) & 0xFFFFFFFFFFFFFFC0);
					memset(coeffs,0,mbtotal * cpmb * sizeof(int16));
				}
				break;

			case 0xFFC4:  // define huffman tables
				{
					int length = get16b(ptr) - 2;
					while(length > 0)
					{
						int d = get8(ptr);
						int tc = d >> 4;
						int n = d & 15;

						uint8 bits[16];
						int total = 0;
						for(int i = 0; i < 16; i++)
						{
							bits[i] = get8(ptr);
							total += bits[i];
						}
						uint8 huffval[256];
						for(int i = 0; i < total; i++)
							huffval[i] = get8(ptr);
						jpeg::Reader::Table* ht = new jpeg::Reader::Table(bits,huffval);

						if(tc)
						{
							if(acht_lib[n])
								delete acht_lib[n];
							acht_lib[n] = ht;
						}
						else
						{
							if(dcht_lib[n])
								delete dcht_lib[n];
							dcht_lib[n] = ht;
						}

						length -= total + 17;
					}
				}
				break;

			case 0xFFD8:  // image start
				break;

			case 0xFFD9:  // image end
				end_found = true;
				break;

			case 0xFFDA:  // scan start
				{
					get16b(ptr);
					int count = get8(ptr);
					jpeg::Reader::Table* acht[4];
					jpeg::Reader::Table* dcht[4];					
					int mask = 0;
					for(int i = 0; i < count; i++)
					{
						int index = get8(ptr) - 1;
						mask |= 1 << index;
						int n = get8(ptr);
						dcht[index] = dcht_lib[n >> 4];
						acht[index] = acht_lib[n & 15];
					}
					int start = get8(ptr);
					int end = get8(ptr);
					uint8 d = get8(ptr);
					bool refine = d & 0xF0;
					int shift = d & 15;

					jpeg::Reader src(ptr);

					int rescnt = resint;
					int eobrun = 0;
					int16 dc[3] = { 0,0,0 };

					int16* block = coeffs;
					for(int i = 0; i < mbtotal; i++)
						jpeg::UnpackMacroblock(block,src,dcht,acht,dc,start,end,shift,refine,eobrun,type,rescnt,resint,mask);

					ptr = src.GetPtr();
				}
				break;

			case 0xFFDB:  // define quantization tables
				{
					int length = get16b(ptr) - 2;

					while(length > 0)
					{
						int d = get8(ptr);
						int n = d & 15;
						if(d >> 4)
						{
							for(int k = 0; k < 64; k++)
								qtable_lib[n][folding[k]] = get16b(ptr);
							length -= 129;
						}
						else
						{
							for(int k = 0; k < 64; k++)
								qtable_lib[n][folding[k]] = get8(ptr);
							length -= 65;
						}
					}
				}
				break;

			case 0xFFDD:  // define restart interval
				{
					get16b(ptr);
					resint = get16b(ptr);
				}
				break;

			case 0xFFE1:  // EXIF segment
				{
					uint length = get16b(ptr) - 2;
					const uint8* nextptr = ptr + length;
					uint32 header = get32b(ptr);
					if(header == 0x45786966)
					{
						// classic Exif

						// skip rest of header
						ptr += 2; 

						// TIFF header
						const uint8* start = ptr;

						// get EXIF endianness
						bool le = get16b(ptr) == 0x4949;

						// skip 0x2A
						le?get16(ptr):get16b(ptr);

						// go to IFD0
						ptr += (le?get32(ptr):get32b(ptr)) - 8;

						// get number of entries
						uint16 entries = (le?get16(ptr):get16b(ptr));
						for(int i = 0; i < entries; i++)
						{
							// get entry
							uint16 tag = le?get16(ptr):get16b(ptr);
							int format = le?get16(ptr):get16b(ptr);
							int components = le?get32(ptr):get32b(ptr);
							uint32 data = le?get32(ptr):get32b(ptr);

							// calculate full data size
							int elsize[] = {0,1,1,2,4,8,1,0,2,4,8,4,8};
							if((format < 0) || (format > 12))
							{
								Debug("UnpackJPEG: EXIF format error\n");
								continue;
							}
							int total = elsize[format] * components;

							// calculate where data starts
							const uint8* dptr = start + data;
							if(total <= 4)
								dptr = ptr - 4;

							// interpret the tags
							switch(tag)
							{
								case 0x0106:  // photometric interpretation (0=inv.mono, 1=mono, 2=RGB, 3=RGB pal, 4=alpha, 5=CMYK, 6=YCbCr, 8=CIELab, 9=ICCLab, 10=ITULab, 32803=CFA, 32844=LogL, 32845=LogLuv, 34892=Linear)
									if(le?get16(dptr):get16b(dptr) == 2)
									{
										if(type != jpeg::TYPE_YUV444)
										{
											Error("UnpackJPEG: RGB only possible with 444 encoding\n");
											return false;
										}
										type = jpeg::TYPE_RGB444;
									}
									break;

								case 0xA001:  // colorspace (1=sRGB, 2=Adobe RGB, FFFD=wide gamut RGB, FFFE=ICC, FFFF=uncalib.)
									break;
							}
						}
					}

					// skip entire block
					ptr = nextptr;
				}
				break;

			case 0xFFC8:
			case 0xFFDC:
			case 0xFFE0:
			case 0xFFE2:
			case 0xFFE3:
			case 0xFFE4:
			case 0xFFE5:
			case 0xFFE6:
			case 0xFFE7:
			case 0xFFE8:
			case 0xFFE9:
			case 0xFFEA:
			case 0xFFEB:
			case 0xFFEC:
			case 0xFFED:
			case 0xFFEE:
			case 0xFFEF:
			case 0xFFF0:
			case 0xFFF1:
			case 0xFFF2:
			case 0xFFF3:
			case 0xFFF4:
			case 0xFFF5:
			case 0xFFF6:
			case 0xFFF7:
			case 0xFFF8:
			case 0xFFF9:
			case 0xFFFA:
			case 0xFFFB:
			case 0xFFFC:
			case 0xFFFD:
			case 0xFFFE:
			case 0xFFFF:
				{
					uint length = get16b(ptr) - 2;
					ptr += length;
				}
				break;

			default:
				Error("UnpackJPEG: marker (%04X) not supported\n",marker);
				return false;
		}

		if(end_found)
			break;
	}

	// dezigzag, dequantize and IDCT
	switch(type)
	{
		case jpeg::TYPE_Y: jpeg::ConvertBlocks(coeffs,mbtotal,jpeg::TYPE_Y,qtable); break;
		case jpeg::TYPE_YUV420: jpeg::ConvertBlocks(coeffs,mbtotal * 6,jpeg::TYPE_YUV420,qtable); break;
		case jpeg::TYPE_YUV422: jpeg::ConvertBlocks(coeffs,mbtotal * 4,jpeg::TYPE_YUV422,qtable); break;
		case jpeg::TYPE_YUV440: jpeg::ConvertBlocks(coeffs,mbtotal * 4,jpeg::TYPE_YUV440,qtable); break;
		case jpeg::TYPE_YUV444: jpeg::ConvertBlocks(coeffs,mbtotal * 3,jpeg::TYPE_YUV444,qtable); break;
		case jpeg::TYPE_RGB444: jpeg::ConvertBlocks(coeffs,mbtotal * 3,jpeg::TYPE_RGB444,qtable); break;
	}

	// draw macroblocks
	switch(type)
	{
		case jpeg::TYPE_Y: jpeg::DrawMacroblocksY(pix,siz,coeffs,mbstride); break;
		case jpeg::TYPE_YUV420: jpeg::DrawMacroblocksYUV420(pix,siz,coeffs,mbstride); break;
		case jpeg::TYPE_YUV422: jpeg::DrawMacroblocksYUV422(pix,siz,coeffs,mbstride); break;
		case jpeg::TYPE_YUV440: jpeg::DrawMacroblocksYUV440(pix,siz,coeffs,mbstride); break;
		case jpeg::TYPE_YUV444: jpeg::DrawMacroblocksYUV444(pix,siz,coeffs,mbstride); break;
		case jpeg::TYPE_RGB444: jpeg::DrawMacroblocksRGB444(pix,siz,coeffs,mbstride); break;
	}

	// cleanup
	for(uint i = 0; i < 4; i++)
		if(dcht_lib[i])
			delete dcht_lib[i];
	for(uint i = 0; i < 4; i++)
		if(acht_lib[i])
			delete acht_lib[i];
	if(unaligned_coeffs)
		delete[] unaligned_coeffs;

	return true;
}


buffer imex PackJPEG(const Image::pixels& pix,int jpegf,int level)
{
	// transfer, FDCT, quantize (according to level) and zigzag the planes into big MCU array
	// if progressive write several layers
	//     YUV DC start at 1
	//     Y AC start 1..5 at 2
	//     V AC start 1..63 at 1
	//     U AC start 1..63 at 1
	//     Y AC start 6..63 at 2
	//     Y AC refine 1..63 at 1
	//     YUV DC refine at 0
	//     V AC refine 1..63 at 0
	//     U AC refine 1..63 at 0
	//     Y AC refine 1..63 at 0
	// if sequential write one big layer
	// before each layer, calculate new huffman tables for optimal use

	return buffer();
}


}
*/