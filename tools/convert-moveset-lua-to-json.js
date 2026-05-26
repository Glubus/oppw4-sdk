const fs = require("fs");
const path = require("path");

const root = process.argv[2];
if (!root) {
  console.error("usage: node tools/convert-moveset-lua-to-json.js <mods-root>");
  process.exit(2);
}

class Parser {
  constructor(source) {
    this.source = source.replace(/^\uFEFF/, "");
    this.index = 0;
  }

  parse() {
    this.skip();
    this.consumeWord("return");
    const value = this.parseValue();
    this.skip();
    return value;
  }

  parseValue() {
    this.skip();
    const char = this.peek();
    if (char === "{") return this.parseTable();
    if (char === '"' || char === "'") return this.parseString();
    if (this.isNumberStart(char)) return this.parseNumber();
    const word = this.parseIdentifier();
    if (word === "true") return true;
    if (word === "false") return false;
    if (word === "nil") return null;
    throw new Error(`unsupported value '${word}' at ${this.index}`);
  }

  parseTable() {
    this.expect("{");
    const array = [];
    const object = {};
    let hasNamed = false;
    let hasArray = false;

    while (true) {
      this.skip();
      if (this.peek() === "}") {
        this.index++;
        break;
      }

      const mark = this.index;
      const key = this.tryParseKey();
      if (key !== null) {
        hasNamed = true;
        object[key] = this.parseValue();
      } else {
        this.index = mark;
        hasArray = true;
        array.push(this.parseValue());
      }

      this.skip();
      if (this.peek() === "," || this.peek() === ";") {
        this.index++;
      }
    }

    if (hasNamed && hasArray) {
      array.forEach((value, i) => {
        object[String(i + 1)] = value;
      });
      return object;
    }
    return hasNamed ? object : array;
  }

  tryParseKey() {
    this.skip();
    const mark = this.index;
    if (this.peek() === "[") {
      this.index++;
      const key = this.parseValue();
      this.skip();
      this.expect("]");
      this.skip();
      if (this.peek() !== "=") {
        this.index = mark;
        return null;
      }
      this.index++;
      return String(key);
    }
    if (!this.isIdentifierStart(this.peek())) return null;
    const key = this.parseIdentifier();
    this.skip();
    if (this.peek() !== "=") {
      this.index = mark;
      return null;
    }
    this.index++;
    return key;
  }

  parseIdentifier() {
    this.skip();
    const start = this.index;
    if (!this.isIdentifierStart(this.peek())) {
      throw new Error(`identifier expected at ${this.index}`);
    }
    this.index++;
    while (this.isIdentifierPart(this.peek())) this.index++;
    return this.source.slice(start, this.index);
  }

  parseNumber() {
    this.skip();
    const start = this.index;
    if (this.peek() === "-") this.index++;
    if (this.source.slice(this.index, this.index + 2).toLowerCase() === "0x") {
      this.index += 2;
      while (/[0-9a-fA-F]/.test(this.peek())) this.index++;
      return this.source.slice(start, this.index);
    }
    while (/[0-9]/.test(this.peek())) this.index++;
    if (this.peek() === ".") {
      this.index++;
      while (/[0-9]/.test(this.peek())) this.index++;
    }
    return Number(this.source.slice(start, this.index));
  }

  parseString() {
    const quote = this.peek();
    this.index++;
    let out = "";
    while (this.index < this.source.length) {
      const char = this.source[this.index++];
      if (char === quote) return out;
      if (char === "\\") {
        const next = this.source[this.index++];
        if (next === "n") out += "\n";
        else if (next === "r") out += "\r";
        else if (next === "t") out += "\t";
        else out += next;
      } else {
        out += char;
      }
    }
    throw new Error("unterminated string");
  }

  consumeWord(word) {
    this.skip();
    if (this.source.slice(this.index, this.index + word.length) !== word) {
      throw new Error(`${word} expected at ${this.index}`);
    }
    this.index += word.length;
  }

  skip() {
    while (this.index < this.source.length) {
      const char = this.peek();
      if (/\s/.test(char)) {
        this.index++;
        continue;
      }
      if (this.source.slice(this.index, this.index + 2) === "--") {
        this.index += 2;
        while (this.index < this.source.length && this.peek() !== "\n") this.index++;
        continue;
      }
      break;
    }
  }

  expect(char) {
    this.skip();
    if (this.peek() !== char) throw new Error(`'${char}' expected at ${this.index}`);
    this.index++;
  }

  peek() {
    return this.source[this.index] || "";
  }

  isIdentifierStart(char) {
    return /[A-Za-z_]/.test(char);
  }

  isIdentifierPart(char) {
    return /[A-Za-z0-9_]/.test(char);
  }

  isNumberStart(char) {
    return char === "-" || /[0-9]/.test(char);
  }
}

function convertFile(file) {
  const source = fs.readFileSync(file, "utf8");
  const parsed = new Parser(source).parse();
  const out = file.replace(/\.lua$/i, ".json");
  fs.writeFileSync(out, `${JSON.stringify(parsed, null, 2)}\n`);
  return out;
}

function wordToU32(word) {
  if (typeof word === "number") return word >>> 0;
  if (typeof word === "string") {
    const trimmed = word.trim();
    if (/^0x/i.test(trimmed)) return Number.parseInt(trimmed.slice(2), 16) >>> 0;
    return Number.parseInt(trimmed, 10) >>> 0;
  }
  if (word && typeof word === "object") {
    if (word.hex !== undefined) return wordToU32(word.hex);
    if (word.u32 !== undefined) return word.u32 >>> 0;
    if (word.i32 !== undefined) return word.i32 >>> 0;
  }
  throw new Error(`unsupported word ${JSON.stringify(word)}`);
}

function wordsToBytes(words) {
  const out = Buffer.alloc(words.length * 4);
  words.forEach((word, i) => out.writeUInt32LE(wordToU32(word), i * 4));
  return out;
}

function alignUp(value, align) {
  return (value + align - 1) & ~(align - 1);
}

function buildPayload(parsed) {
  if (parsed.payload_hex) {
    return Buffer.from(parsed.payload_hex.replace(/\s+/g, ""), "hex");
  }
  if (!Array.isArray(parsed.sections)) {
    throw new Error("moveset needs sections");
  }
  const sectionCount = parsed.section_count || 18;
  const sections = Array.from({ length: sectionCount }, () => Buffer.alloc(0));
  for (const section of parsed.sections) {
    if (section.index >= sectionCount) {
      throw new Error(`section index ${section.index} >= ${sectionCount}`);
    }
    if (Array.isArray(section.words) && section.words.length > 0) {
      sections[section.index] = wordsToBytes(section.words);
      continue;
    }
    const records = section.records || [];
    sections[section.index] = Buffer.concat(records.map(wordsToBytes));
  }

  const headerLen = alignUp(4 + sectionCount * 4 + 4, 0x10);
  const chunks = [Buffer.alloc(headerLen)];
  chunks[0].writeUInt32LE(sectionCount, 0);
  let cursor = headerLen;
  sections.forEach((bytes, index) => {
    chunks[0].writeUInt32LE(cursor, 4 + index * 4);
    chunks.push(bytes);
    cursor += bytes.length;
    const padding = (0x10 - (cursor % 0x10)) % 0x10;
    if (padding > 0) {
      chunks.push(Buffer.alloc(padding));
      cursor += padding;
    }
  });
  return Buffer.concat(chunks);
}

function convertFileToBin(file) {
  const source = fs.readFileSync(file, "utf8");
  const parsed = new Parser(source).parse();
  const out = file.replace(/\.lua$/i, ".bin");
  fs.writeFileSync(out, buildPayload(parsed));
  return out;
}

function convertJsonFileToBin(file) {
  const parsed = JSON.parse(fs.readFileSync(file, "utf8"));
  const out = file.replace(/\.json$/i, ".bin");
  fs.writeFileSync(out, buildPayload(parsed));
  return out;
}

let converted = 0;
let convertedBin = 0;
let patched = 0;

function walkDirs(dir) {
  const dirs = [dir];
  for (const dirent of fs.readdirSync(dir, { withFileTypes: true })) {
    if (dirent.isDirectory()) {
      dirs.push(...walkDirs(path.join(dir, dirent.name)));
    }
  }
  return dirs;
}

for (const dir of walkDirs(root)) {
  if (!path.basename(dir).endsWith("_moveset")) continue;
  const modLua = path.join(dir, "mod.lua");
  if (!fs.existsSync(modLua)) continue;
  let script = fs.readFileSync(modLua, "utf8");
  const refs = [...script.matchAll(/moveset_patcher\.load_patch\(\s*["']([^"']+\.lua)["']\s*\)/g)];
  for (const ref of refs) {
    const luaName = ref[1];
    const luaPath = path.join(dir, luaName);
    if (!fs.existsSync(luaPath)) continue;
    convertFile(luaPath);
    const binPath = convertFileToBin(luaPath);
    converted++;
    convertedBin++;
    const binName = path.basename(binPath);
    script = script.replace(
      ref[0],
      `moveset_patcher.patch({ payload_file = "${binName}" })`,
    );
  }
  const jsonRefs = [...script.matchAll(/payload_file = "([^"]+\.json)"/g)];
  for (const ref of jsonRefs) {
    const jsonPath = path.join(dir, ref[1]);
    if (fs.existsSync(jsonPath)) {
      convertJsonFileToBin(jsonPath);
      convertedBin++;
    }
  }
  script = script.replace(/payload_file = "([^"]+)\.json"/g, 'payload_file = "$1.bin"');
  if (refs.length > 0) {
    fs.writeFileSync(modLua, script);
    patched++;
  } else if (/payload_file = "[^"]+\.bin"/.test(script)) {
    fs.writeFileSync(modLua, script);
  }
}

for (const dir of walkDirs(root)) {
  for (const dirent of fs.readdirSync(dir, { withFileTypes: true })) {
    if (dirent.isFile() && dirent.name.endsWith("_moveset.json")) {
      convertJsonFileToBin(path.join(dir, dirent.name));
      convertedBin++;
    }
  }
}

console.log(`converted_json=${converted} converted_bin=${convertedBin} patched_mods=${patched}`);
