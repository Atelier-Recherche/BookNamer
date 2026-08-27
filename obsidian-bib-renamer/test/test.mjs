import assert from 'node:assert';

// Inline test to verify logic cleanly
import { parseFilename, extractIsbn, extractDoi, extractYear, sanitizeFilename } from '../src/heuristics.ts';

function formatAuthor(name) {
  const parts = name.trim().split(/\s+/);
  if (parts.length >= 2) {
    const first = parts[0];
    const rest = parts.slice(1).join(' ');
    return `${rest}, ${first}`;
  }
  return name.trim();
}

function escapeRegex(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function replaceOptional(tpl, variable, value) {
  if (!value) {
    return tpl
      .replace(new RegExp(`\\[${escapeRegex(variable)}\\]`, 'g'), '')
      .replace(new RegExp(`\\(${escapeRegex(variable)}\\)`, 'g'), '')
      .replace(new RegExp(escapeRegex(variable), 'g'), '');
  }
  return tpl.replace(new RegExp(escapeRegex(variable), 'g'), value);
}

function cleanEmptyBrackets(text) {
  let prev = text;
  while (true) {
    let cleaned = prev.replace(/\[\s*\]|\(\s*\)|\{\s*\}/g, '');
    cleaned = cleaned.replace(/\s{2,}/g, ' ');
    if (cleaned === prev) break;
    prev = cleaned;
  }
  return prev;
}

function applyTemplate(template, metadata, extension) {
  const auteur = metadata.authors.length > 0
    ? formatAuthor(metadata.authors[0])
    : 'Auteur inconnu';
  const titre = metadata.title ?? 'Sans titre';
  const date = metadata.date ?? 's.d.';
  const editeur = metadata.publisher ?? '';
  const isbn = metadata.isbn ?? '';
  const doi = metadata.doi ?? '';
  const ext = extension.replace(/^\.+/, '');

  let result = template.pattern;
  result = result.replace(/{auteur}/g, auteur);
  result = result.replace(/{titre}/g, titre);
  result = result.replace(/{date}/g, date);
  result = result.replace(/{extension}/g, ext);

  result = replaceOptional(result, '{editeur}', editeur);
  result = replaceOptional(result, '{isbn}', isbn);
  result = replaceOptional(result, '{doi}', doi);

  result = cleanEmptyBrackets(result).trim();

  const finalName = `${result}.${ext}`;
  return sanitizeFilename(finalName);
}

console.log('--- Testing Obsidian Bib Renamer Heuristics & Templates ---');

// 1. Test ISBN Extraction
assert.strictEqual(extractIsbn('La vérité nue 9782021478006 blah'), '9782021478006');
assert.strictEqual(extractIsbn('Book with isbn: 978-2-02-147800-6'), '9782021478006');
console.log('✓ ISBN extraction tests passed');

// 2. Test DOI Extraction
assert.strictEqual(
  extractDoi('Some article 10.1016/j.cell.2022.01.001 here'),
  '10.1016/j.cell.2022.01.001'
);
console.log('✓ DOI extraction tests passed');

// 3. Test Year Extraction
assert.strictEqual(extractYear('Blumenberg 2022 Seuil'), '2022');
console.log('✓ Year extraction tests passed');

// 4. Test Noise Cleaning and Filename Parsing
const parsed = parseFilename("La vérité nue d3b07384d113edec49eaa6238ad5ff00 anna's archive 2022.epub");
assert(!parsed.cleaned.includes('d3b07384'));
assert(!parsed.cleaned.toLowerCase().includes('anna'));
assert.strictEqual(parsed.year, '2022');
console.log('✓ Heuristic cleaning tests passed');

// 5. Test Template Engine
const template = {
  id: 'test',
  name: 'Test Template',
  pattern: '{auteur} - {titre} ({date}) [{editeur}]',
  is_default: true,
};

const metadata = {
  title: 'La vérité nue',
  authors: ['Hans Blumenberg'],
  date: '2022',
  publisher: 'Seuil Jeunesse',
  isbn: '9782021478006',
  doi: null,
  source: 'openlibrary',
  confidence: 'high',
};

const result = applyTemplate(template, metadata, 'epub');
assert(result.includes('Blumenberg, Hans'));
assert(result.includes('La vérité nue'));
assert(result.includes('2022'));
assert(result.includes('Seuil Jeunesse'));
assert(result.endsWith('.epub'));
console.log('✓ Template application tests passed:', result);

// 6. Test Empty Optional Fields Cleanup
const metaNoPub = { ...metadata, publisher: null };
const resultNoPub = applyTemplate(template, metaNoPub, 'pdf');
assert(!resultNoPub.includes('[]'));
assert(resultNoPub.endsWith('.pdf'));
console.log('✓ Empty bracket cleanup tests passed:', resultNoPub);

// 7. Test OS Sanitization
assert.strictEqual(
  sanitizeFilename('Titre: Avec / Slashes * Partout.pdf'),
  'Titre Avec Slashes Partout.pdf'
);
console.log('✓ Sanitization tests passed');

console.log('\nAll tests passed successfully! 🎉');
