/**
 * Simple test runner for CLI integration tests
 * Runs tests directly with Mocha without VS Code test runner
 */

const Mocha = require('mocha');
const path = require('path');
const glob = require('glob');

const mocha = new Mocha({
    ui: 'tdd',
    color: true,
    timeout: 10000
});

// Add test files
const testFiles = [
    'out/test/suite/platform-detector.test.js',
    'out/test/suite/binary-resolver.test.js',
    'out/test/suite/binary-verifier.test.js',
    'out/test/suite/binary-resolver.property.test.js'
];

console.log('Running CLI Integration Tests...\n');

testFiles.forEach(file => {
    const fullPath = path.join(__dirname, file);
    console.log(`Adding test file: ${file}`);
    mocha.addFile(fullPath);
});

console.log('\n');

// Run tests
mocha.run(failures => {
    if (failures > 0) {
        console.error(`\n${failures} test(s) failed`);
        process.exit(1);
    } else {
        console.log('\nAll tests passed!');
        process.exit(0);
    }
});
