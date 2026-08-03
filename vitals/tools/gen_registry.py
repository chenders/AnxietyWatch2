#!/usr/bin/env python3

import sys
import os
import tempfile
import tomllib
import json

def main():
    if len(sys.argv) > 1 and sys.argv[1] == '--check':
        check_mode()
    else:
        generate_mode()

def check_mode():
    # Load the registry
    with open('vitals/registry.toml', 'rb') as f:
        registry = tomllib.load(f)
    
    # Create temporary directory
    with tempfile.TemporaryDirectory() as temp_dir:
        # Generate files in temp directory
        generate_files(registry, temp_dir)
        
        # Compare with existing files
        generated_files = [
            'vitals_types.h',
            'registry.sql',
            'VitalsRegistry.swift',
            'registry.json'
        ]
        
        for filename in generated_files:
            temp_path = os.path.join(temp_dir, filename)
            actual_path = os.path.join('vitals/generated', filename)
            
            # Read generated content
            with open(temp_path, 'r') as f:
                temp_content = f.read()
            
            # Read actual content
            with open(actual_path, 'r') as f:
                actual_content = f.read()
            
            # Compare
            if temp_content != actual_content:
                print(f"Diff detected in {filename}")
                sys.exit(1)
        
        print("All generated files are up to date")
        sys.exit(0)

def generate_mode():
    # Load the registry
    with open('vitals/registry.toml', 'rb') as f:
        registry = tomllib.load(f)
    
    # Generate files
    generate_files(registry, 'vitals/generated')

def generate_files(registry, output_dir):
    # Generate C header
    generate_c_header(registry, os.path.join(output_dir, 'vitals_types.h'))
    
    # Generate SQL
    generate_sql(registry, os.path.join(output_dir, 'registry.sql'))
    
    # Generate Swift
    generate_swift(registry, os.path.join(output_dir, 'VitalsRegistry.swift'))
    
    # Generate JSON
    generate_json(registry, os.path.join(output_dir, 'registry.json'))

def generate_c_header(registry, output_path):
    lines = [
        '/* GENERATED FROM registry.toml — DO NOT EDIT */',
        '#ifndef VITALS_TYPES_H',
        '#define VITALS_TYPES_H',
        '',
        '#ifdef __cplusplus',
        'extern "C" {',
        '#endif',
        ''
    ]
    
    # Generate type enums
    lines.append('// Vitals types')
    lines.append('typedef enum {')
    type_entries = []
    if 'type' in registry:
        # Sort by ID for consistent ordering
        sorted_types = sorted(registry['type'].items(), key=lambda item: item[1]['id'])
        for type_name, data in sorted_types:
            if isinstance(data, dict) and 'id' in data:
                enum_entry = f'    VITALS_TYPE_{type_name.upper()} = {data["id"]}'
                type_entries.append(enum_entry)
    lines.append(',\n'.join(type_entries))
    lines.append('} vitals_type_t;')
    
    lines.append('')
    
    # Generate source enums
    lines.append('')
    lines.append('// Vitals sources')
    lines.append('typedef enum {')
    source_entries = []
    if 'source' in registry:
        # Sort by ID for consistent ordering
        sorted_sources = sorted(registry['source'].items(), key=lambda item: item[1]['id'])
        for source_name, data in sorted_sources:
            if isinstance(data, dict) and 'id' in data:
                enum_entry = f'    VITALS_SOURCE_{source_name.upper()} = {data["id"]}'
                # Handle special case for ad_ua651ble
                if source_name == 'ad_ua651ble':
                    enum_entry = f'    VITALS_SOURCE_AD_UA651BLE = {data["id"]}'
                source_entries.append(enum_entry)
    lines.append(',\n'.join(source_entries))
    lines.append('} vitals_source_t;')
    
    lines.extend([
        '',
        '// Type information functions',
        'typedef struct {',
        '    double lo;',
        '    double hi;',
        '} vitals_range_t;',
        '',
        'vitals_range_t vitals_type_range(int type_id);',
        'const char* vitals_type_unit(int type_id);',
        '',
        '#ifdef __cplusplus',
        '}',
        '#endif',
        '',
        '#endif // VITALS_TYPES_H'
    ])
    
    with open(output_path, 'w') as f:
        f.write('\n'.join(lines) + '\n')

def generate_sql(registry, output_path):
    lines = [
        '/* GENERATED FROM registry.toml — DO NOT EDIT */',
        '',
        '-- Vitals types lookup table',
        'CREATE TABLE IF NOT EXISTS vitals_type (',
        '    id INTEGER PRIMARY KEY,',
        '    name TEXT NOT NULL UNIQUE,',
        '    unit TEXT NOT NULL,',
        '    range_lo REAL NOT NULL,',
        '    range_hi REAL NOT NULL,',
        '    precision REAL NOT NULL',
        ');',
        ''
    ]
    
    # Generate type INSERT statements
    for name, data in registry.items():
        if name.startswith('type.'):
            type_name = name.split('.')[1]
            range_lo, range_hi = data['range']
            lines.append(f"INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)")
            lines.append(f"VALUES ({data['id']}, '{type_name}', '{data['unit']}', {range_lo}, {range_hi}, {data['precision']});")
    
    lines.extend([
        '',
        '-- Vitals sources lookup table',
        'CREATE TABLE IF NOT EXISTS vitals_source (',
        '    id INTEGER PRIMARY KEY,',
        '    name TEXT NOT NULL UNIQUE,',
        '    transport TEXT NOT NULL',
        ');',
        ''
    ])
    
    # Generate source INSERT statements
    for name, data in registry.items():
        if name.startswith('source.'):
            source_name = name.split('.')[1]
            transport = data['transport']
            lines.append(f"INSERT OR REPLACE INTO vitals_source (id, name, transport)")
            lines.append(f"VALUES ({data['id']}, '{source_name}', '{transport}');")
    
    lines.extend([
        '',
        '-- Add foreign key constraints to samples table',
        'CREATE TABLE IF NOT EXISTS samples (',
        '    id INTEGER PRIMARY KEY,',
        '    timestamp INTEGER NOT NULL,',
        '    type INTEGER NOT NULL,',
        '    source INTEGER NOT NULL,',
        '    value REAL NOT NULL,',
        '    FOREIGN KEY(type) REFERENCES vitals_type(id),',
        '    FOREIGN KEY(source) REFERENCES vitals_source(id),',
        '    CHECK(type IN (SELECT id FROM vitals_type)),',
        '    CHECK(source IN (SELECT id FROM vitals_source))',
        ');'
    ])
    
    with open(output_path, 'w') as f:
        f.write('\n'.join(lines) + '\n')

def generate_swift(registry, output_path):
    lines = [
        '/* GENERATED FROM registry.toml — DO NOT EDIT */',
        '',
        'import Foundation',
        '',
        'public enum VitalsType: Int32 {'
    ]
    
    # Generate type cases
    if 'type' in registry:
        for type_name, data in registry['type'].items():
            if isinstance(data, dict) and 'id' in data:
                swift_name = type_name.upper()
                # Special handling for bp_* types
                if type_name.startswith('bp_'):
                    swift_name = ''.join(word.capitalize() for word in type_name.split('_'))
                lines.append(f'    case {swift_name} = {data["id"]}')
    
    lines.extend([
        '}',
        '',
        'public enum VitalsSource: Int32 {'
    ])
    
    # Generate source cases
    if 'source' in registry:
        for source_name, data in registry['source'].items():
            if isinstance(data, dict) and 'id' in data:
                swift_name = source_name.upper()
                # Special handling for ad_ua651ble
                if source_name == 'ad_ua651ble':
                    swift_name = 'AD_UA651BLE'
                lines.append(f'    case {swift_name} = {data["id"]}')
    
    lines.extend([
        '}',
        ''
    ])
    
    with open(output_path, 'w') as f:
        f.write('\n'.join(lines) + '\n')

def generate_json(registry, output_path):
    # Just write the registry as JSON
    with open(output_path, 'w') as f:
        json.dump(registry, f, indent=2)

if __name__ == '__main__':
    main()