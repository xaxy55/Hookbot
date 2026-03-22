/**
 * Minimal QR Code generator (SVG) — no external dependencies.
 * Supports alphanumeric mode, error correction level L, versions 1-10.
 */

// Galois field math for Reed-Solomon
const GF256_EXP = new Uint8Array(512);
const GF256_LOG = new Uint8Array(256);
(() => {
  let x = 1;
  for (let i = 0; i < 255; i++) {
    GF256_EXP[i] = x;
    GF256_LOG[x] = i;
    x = (x << 1) ^ (x >= 128 ? 0x11d : 0);
  }
  for (let i = 255; i < 512; i++) GF256_EXP[i] = GF256_EXP[i - 255];
})();

function gfMul(a: number, b: number): number {
  if (a === 0 || b === 0) return 0;
  return GF256_EXP[GF256_LOG[a] + GF256_LOG[b]];
}

function rsEncode(data: number[], ecCount: number): number[] {
  const gen: number[] = new Array(ecCount + 1).fill(0);
  gen[0] = 1;
  for (let i = 0; i < ecCount; i++) {
    for (let j = gen.length - 1; j >= 1; j--) {
      gen[j] = gen[j - 1] ^ gfMul(gen[j], GF256_EXP[i]);
    }
    gen[0] = gfMul(gen[0], GF256_EXP[i]);
  }
  const rem = new Array(ecCount).fill(0);
  for (const b of data) {
    const lead = b ^ rem[0];
    for (let i = 0; i < ecCount - 1; i++) rem[i] = rem[i + 1] ^ gfMul(gen[ecCount - 1 - i], lead);
    rem[ecCount - 1] = gfMul(gen[0], lead);
  }
  return rem;
}

// QR constants
const ALIGNMENT_POSITIONS: number[][] = [
  [], [], [6, 18], [6, 22], [6, 26], [6, 30], [6, 34],
  [6, 22, 38], [6, 24, 42], [6, 26, 46], [6, 28, 50],
];

const EC_CODEWORDS_L = [0, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18];
const DATA_CODEWORDS_L = [0, 19, 34, 55, 80, 108, 136, 156, 194, 232, 274];
const FORMAT_BITS_L = [0x77c4, 0x72f3, 0x7daa, 0x789d, 0x662f, 0x6318, 0x6c41, 0x6976,
  0x5412, 0x5125, 0x5e7c, 0x5b4b, 0x45f9, 0x40ce, 0x4f97, 0x4aa0];

function getVersion(byteLength: number): number {
  for (let v = 1; v <= 10; v++) {
    if (DATA_CODEWORDS_L[v] >= byteLength) return v;
  }
  return 10;
}

function encodeBytes(data: string): number[] {
  // Byte mode encoding
  const bytes = new TextEncoder().encode(data);
  const version = getVersion(bytes.length + 3);
  const totalDataCW = DATA_CODEWORDS_L[version];

  const bits: number[] = [];
  const pushBits = (val: number, len: number) => {
    for (let i = len - 1; i >= 0; i--) bits.push((val >> i) & 1);
  };

  // Mode indicator: byte = 0100
  pushBits(0b0100, 4);
  // Character count (8 bits for versions 1-9, 16 for 10+)
  pushBits(bytes.length, version <= 9 ? 8 : 16);
  for (const b of bytes) pushBits(b, 8);
  // Terminator
  pushBits(0, Math.min(4, totalDataCW * 8 - bits.length));
  // Pad to byte boundary
  while (bits.length % 8 !== 0) bits.push(0);
  // Pad codewords
  const padBytes = [0xec, 0x11];
  let padIdx = 0;
  while (bits.length < totalDataCW * 8) {
    pushBits(padBytes[padIdx % 2], 8);
    padIdx++;
  }

  const codewords: number[] = [];
  for (let i = 0; i < bits.length; i += 8) {
    let byte = 0;
    for (let j = 0; j < 8; j++) byte = (byte << 1) | bits[i + j];
    codewords.push(byte);
  }

  return codewords;
}

function buildMatrix(data: string): boolean[][] {
  const codewords = encodeBytes(data);
  const byteLen = new TextEncoder().encode(data).length + 3;
  const version = getVersion(byteLen);
  const size = version * 4 + 17;
  const ecCount = EC_CODEWORDS_L[version];

  // Generate error correction
  const ecBytes = rsEncode(codewords, ecCount);
  const allBytes = [...codewords, ...ecBytes];

  // Create matrix (-1 = unset)
  const matrix: number[][] = Array.from({ length: size }, () => new Array(size).fill(-1));

  // Place finder patterns
  const placeFinder = (row: number, col: number) => {
    for (let r = -1; r <= 7; r++) {
      for (let c = -1; c <= 7; c++) {
        const rr = row + r, cc = col + c;
        if (rr < 0 || rr >= size || cc < 0 || cc >= size) continue;
        if (r >= 0 && r <= 6 && c >= 0 && c <= 6) {
          matrix[rr][cc] = (r === 0 || r === 6 || c === 0 || c === 6 ||
            (r >= 2 && r <= 4 && c >= 2 && c <= 4)) ? 1 : 0;
        } else {
          matrix[rr][cc] = 0;
        }
      }
    }
  };
  placeFinder(0, 0);
  placeFinder(0, size - 7);
  placeFinder(size - 7, 0);

  // Timing patterns
  for (let i = 8; i < size - 8; i++) {
    if (matrix[6][i] === -1) matrix[6][i] = i % 2 === 0 ? 1 : 0;
    if (matrix[i][6] === -1) matrix[i][6] = i % 2 === 0 ? 1 : 0;
  }

  // Alignment patterns
  if (version >= 2) {
    const positions = ALIGNMENT_POSITIONS[version];
    for (const r of positions) {
      for (const c of positions) {
        if (matrix[r][c] !== -1) continue;
        for (let dr = -2; dr <= 2; dr++) {
          for (let dc = -2; dc <= 2; dc++) {
            matrix[r + dr][c + dc] = (Math.abs(dr) === 2 || Math.abs(dc) === 2 || (dr === 0 && dc === 0)) ? 1 : 0;
          }
        }
      }
    }
  }

  // Dark module
  matrix[size - 8][8] = 1;

  // Reserve format info areas
  for (let i = 0; i < 8; i++) {
    if (matrix[8][i] === -1) matrix[8][i] = 0;
    if (matrix[8][size - 1 - i] === -1) matrix[8][size - 1 - i] = 0;
    if (matrix[i][8] === -1) matrix[i][8] = 0;
    if (matrix[size - 1 - i][8] === -1) matrix[size - 1 - i][8] = 0;
  }
  if (matrix[8][8] === -1) matrix[8][8] = 0;

  // Place data bits
  const dataBits: number[] = [];
  for (const b of allBytes) {
    for (let i = 7; i >= 0; i--) dataBits.push((b >> i) & 1);
  }

  let bitIdx = 0;
  let upward = true;
  for (let right = size - 1; right >= 1; right -= 2) {
    if (right === 6) right = 5; // Skip timing column
    const rows = upward ? Array.from({ length: size }, (_, i) => size - 1 - i) : Array.from({ length: size }, (_, i) => i);
    for (const row of rows) {
      for (const col of [right, right - 1]) {
        if (matrix[row][col] === -1) {
          matrix[row][col] = bitIdx < dataBits.length ? dataBits[bitIdx++] : 0;
        }
      }
    }
    upward = !upward;
  }

  // Apply mask pattern 0 (checkerboard: (row + col) % 2 === 0)
  for (let r = 0; r < size; r++) {
    for (let c = 0; c < size; c++) {
      // Only mask data areas (not function patterns)
      if (isDataModule(r, c, size, version)) {
        if ((r + c) % 2 === 0) {
          matrix[r][c] ^= 1;
        }
      }
    }
  }

  // Write format info (mask pattern 0, EC level L)
  const formatBits = FORMAT_BITS_L[0]; // mask 0
  const formatPositions = getFormatPositions(size);
  for (let i = 0; i < 15; i++) {
    const bit = (formatBits >> (14 - i)) & 1;
    const [r1, c1] = formatPositions.first[i];
    const [r2, c2] = formatPositions.second[i];
    matrix[r1][c1] = bit;
    matrix[r2][c2] = bit;
  }

  return matrix.map(row => row.map(cell => cell === 1));
}

function isDataModule(row: number, col: number, size: number, version: number): boolean {
  // Finder patterns + separators
  if (row <= 8 && col <= 8) return false;
  if (row <= 8 && col >= size - 8) return false;
  if (row >= size - 8 && col <= 8) return false;
  // Timing
  if (row === 6 || col === 6) return false;
  // Dark module
  if (row === size - 8 && col === 8) return false;
  // Alignment patterns
  if (version >= 2) {
    const positions = ALIGNMENT_POSITIONS[version];
    for (const ar of positions) {
      for (const ac of positions) {
        // Skip alignment patterns that overlap with finder patterns
        if ((ar <= 8 && ac <= 8) || (ar <= 8 && ac >= size - 8) || (ar >= size - 8 && ac <= 8)) continue;
        if (Math.abs(row - ar) <= 2 && Math.abs(col - ac) <= 2) return false;
      }
    }
  }
  return true;
}

function getFormatPositions(size: number) {
  const first: [number, number][] = [];
  const second: [number, number][] = [];

  // First copy: around top-left finder
  for (let i = 0; i <= 5; i++) first.push([8, i]);
  first.push([8, 7]);
  first.push([8, 8]);
  first.push([7, 8]);
  for (let i = 5; i >= 0; i--) first.push([i, 8]);

  // Second copy: split between bottom-left and top-right
  for (let i = size - 1; i >= size - 7; i--) second.push([8, i]);
  for (let i = size - 8; i < size; i++) second.push([i, 8]);

  return { first, second };
}

interface QRCodeProps {
  value: string;
  size?: number;
  fgColor?: string;
  bgColor?: string;
  className?: string;
}

export default function QRCode({ value, size = 200, fgColor = 'currentColor', bgColor = 'transparent', className }: QRCodeProps) {
  const matrix = buildMatrix(value);
  const moduleCount = matrix.length;
  const quietZone = 4;
  const total = moduleCount + quietZone * 2;

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${total} ${total}`}
      className={className}
      xmlns="http://www.w3.org/2000/svg"
    >
      <rect width={total} height={total} fill={bgColor} />
      {matrix.map((row, y) =>
        row.map((cell, x) =>
          cell ? (
            <rect
              key={`${y}-${x}`}
              x={x + quietZone}
              y={y + quietZone}
              width={1}
              height={1}
              fill={fgColor}
            />
          ) : null
        )
      )}
    </svg>
  );
}
