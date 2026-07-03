/**
 * Simple test script to verify project structure
 * This script checks if all required files exist and are properly configured
 */

import * as fs from 'fs';
import * as path from 'path';

const requiredFiles = [
  'package.json',
  'Cargo.toml',
  'tsconfig.json',
  'vite.config.ts',
  'src/lib.rs',
  'src/index.ts',
  'src/drm_renderer.rs',
  'src/texture_manager.rs',
  'src/transform.rs',
  'src/types/native.d.ts',
  'README.md',
  'LICENSE',
  '.gitignore',
  '.npmignore',
  'examples/basic.ts',
  'examples/animation.ts',
];

function checkFile(filePath: string): boolean {
  const fullPath = path.join(__dirname, '..', filePath);
  const exists = fs.existsSync(fullPath);
  
  if (!exists) {
    console.error(`❌ Missing: ${filePath}`);
    return false;
  }
  
  console.log(`✅ Found: ${filePath}`);
  return true;
}

function checkPackageJson(): boolean {
  try {
    const packagePath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf-8'));
    
    const requiredFields = ['name', 'version', 'main', 'types', 'scripts', 'license'];
    let valid = true;
    
    for (const field of requiredFields) {
      if (!packageJson[field]) {
        console.error(`❌ Missing package.json field: ${field}`);
        valid = false;
      }
    }
    
    if (valid) {
      console.log('✅ package.json is valid');
    }
    
    return valid;
  } catch (error) {
    console.error('❌ Error reading package.json:', error);
    return false;
  }
}

function checkCargoToml(): boolean {
  try {
    const cargoPath = path.join(__dirname, '..', 'Cargo.toml');
    const content = fs.readFileSync(cargoPath, 'utf-8');
    
    const requiredDeps = ['neon', 'drm', 'gbm', 'nalgebra'];
    let valid = true;
    
    for (const dep of requiredDeps) {
      if (!content.includes(dep)) {
        console.error(`❌ Missing Cargo.toml dependency: ${dep}`);
        valid = false;
      }
    }
    
    if (valid) {
      console.log('✅ Cargo.toml is valid');
    }
    
    return valid;
  } catch (error) {
    console.error('❌ Error reading Cargo.toml:', error);
    return false;
  }
}

function checkSharedTextureTypes(): boolean {
  try {
    const typesPath = path.join(__dirname, '..', 'src', 'types', 'native.d.ts');
    const content = fs.readFileSync(typesPath, 'utf-8');
    
    const requiredTypes = [
      'SharedTextureHandle',
      'SharedTextureImportTextureInfo',
      'NativePixmap',
      'SharedTexturePlane',
      'PixelFormat'
    ];
    
    let valid = true;
    
    for (const type of requiredTypes) {
      if (!content.includes(type)) {
        console.error(`❌ Missing type definition: ${type}`);
        valid = false;
      }
    }
    
    if (valid) {
      console.log('✅ SharedTexture types are defined');
    }
    
    return valid;
  } catch (error) {
    console.error('❌ Error checking types:', error);
    return false;
  }
}

function main() {
  console.log('=== Project Structure Test ===\n');
  
  let allValid = true;
  
  // Check required files
  console.log('Checking required files...');
  for (const file of requiredFiles) {
    if (!checkFile(file)) {
      allValid = false;
    }
  }
  
  console.log('\nChecking configuration files...');
  
  // Check package.json
  if (!checkPackageJson()) {
    allValid = false;
  }
  
  // Check Cargo.toml
  if (!checkCargoToml()) {
    allValid = false;
  }
  
  // Check SharedTexture types
  if (!checkSharedTextureTypes()) {
    allValid = false;
  }
  
  console.log('\n=== Test Results ===');
  
  if (allValid) {
    console.log('✅ All tests passed! Project structure is valid.');
    process.exit(0);
  } else {
    console.log('❌ Some tests failed. Please check the errors above.');
    process.exit(1);
  }
}

main();