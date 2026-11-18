// Run on GitHub after the client's frozen pnpm install. Resolve through the
// application's dependency graph so this checks the exact compiler it ships,
// including pnpm's isolated node_modules layout, without another dependency.
import { readFileSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const clientRequire = createRequire(join(root, 'client/package.json'));
const vueI18nRequire = createRequire(clientRequire.resolve('vue-i18n'));
const coreRequire = createRequire(vueI18nRequire.resolve('@intlify/core-base'));
const { baseCompile } = coreRequire('@intlify/message-compiler');
const { version } = coreRequire('@intlify/message-compiler/package.json');
const installedVersion = clientRequire('vue-i18n/package.json').version;
const pinnedVersion = clientRequire('./package.json').dependencies['vue-i18n'];
if (version !== installedVersion || installedVersion !== pinnedVersion || typeof baseCompile !== 'function') {
    throw new Error(`Unexpected Vue I18n compiler: pinned=${pinnedVersion}, installed=${installedVersion}, compiler=${version}`);
}

const catalogsRoot = join(root, 'client/src/locales');
const failures = [];
let messageCount = 0;
let catalogCount = 0;

for (const locale of readdirSync(catalogsRoot, { withFileTypes: true }).filter(entry => entry.isDirectory())
    .map(entry => entry.name).sort()) {
    const directory = join(catalogsRoot, locale);
    for (const filename of readdirSync(directory).filter(name => name.endsWith('.json')).sort()) {
        const path = join(directory, filename);
        const label = relative(root, path).replaceAll('\\', '/');
        let catalog;
        try {
            catalog = JSON.parse(readFileSync(path, 'utf8'));
            if (!catalog || typeof catalog !== 'object' || Array.isArray(catalog)) {
                throw new Error('Catalog must be a flat object of message strings');
            }
        } catch (error) {
            failures.push(`${label}: ${error.message}`);
            continue;
        }
        catalogCount += 1;
        for (const [key, message] of Object.entries(catalog)) {
            messageCount += 1;
            if (typeof message !== 'string') {
                failures.push(`${label} / ${key}: Message must be a string`);
                continue;
            }
            const errors = [];
            try {
                // The pinned runtime also uses baseCompile with jit:true.
                // Parse all plural alternatives and interpolations; do not
                // evaluate generated JavaScript or depend on UI route coverage.
                baseCompile(message, { jit: true, location: true, onError: error => errors.push(error) });
            } catch (error) {
                if (!errors.includes(error)) errors.push(error);
            }
            for (const error of errors) {
                const start = error.location?.start;
                const position = start ? ` (message ${start.line}:${start.column})` : '';
                failures.push(`${label} / ${key}${position}: ${error.message}`);
            }
        }
    }
}

if (messageCount === 0) failures.push('No catalog messages were found');
if (failures.length) {
    for (const failure of failures) console.error(failure);
    console.error(`Vue I18n ${version}: ${failures.length} compiler/catalog errors across ${messageCount} messages.`);
    process.exitCode = 1;
} else {
    console.log(`Vue I18n ${version}: compiled all ${messageCount} messages in ${catalogCount} catalogs.`);
}
