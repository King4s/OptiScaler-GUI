#!/usr/bin/env python3
"""
Test script for the enhanced GitHub functionality in OptiScaler Manager
"""

import sys
import os
sys.path.append('src')

from optiscaler.manager_new import OptiScalerManager

def test_github_integration():
    print("=== Testing Enhanced GitHub Integration ===")
    
    try:
        manager = OptiScalerManager()
        print("✓ OptiScalerManager initialized successfully")
        
        # Test release info
        print("\n1. Testing release info...")
        release_info = manager.get_release_info()
        if release_info:
            print(f"✓ Latest release: {release_info['name']}")
            print(f"  - Version: {release_info['tag_name']}")
            print(f"  - Published: {release_info['published_at']}")
            print(f"  - Assets: {len(release_info['assets'])} files")
            print(f"  - Prerelease: {release_info['prerelease']}")
        else:
            print("✗ Failed to fetch release info")
            return False
        
        # Test GitHub connection
        print("\n2. Testing GitHub connection...")
        connection_test = manager.test_github_connection()
        if connection_test['connected']:
            print(f"✓ GitHub API connected")
            print(f"  - Rate limit: {connection_test['rate_limit']}/hour")
            print(f"  - Remaining: {connection_test['remaining']}")
        else:
            print(f"✗ GitHub connection failed: {connection_test.get('error', 'Unknown error')}")
        
        # Test version comparison
        print("\n3. Testing version comparison...")
        test_versions = [
            ("v0.7.6", "v0.7.7", True),
            ("v0.7.7", "v0.7.7", False),
            ("v0.7.8", "v0.7.7", False),
            ("0.7.6", "0.7.7-pre9", True)
        ]
        
        for current, latest, expected in test_versions:
            result = manager._compare_versions(current, latest)
            status = "✓" if result == expected else "✗"
            print(f"  {status} {current} vs {latest}: {'needs update' if result else 'up to date'}")
        
        # Test update check
        print("\n4. Testing update check...")
        update_info = manager.check_for_updates("v0.7.6")
        if update_info:
            if update_info['update_available']:
                print(f"✓ Update available: {update_info['current_version']} -> {update_info['latest_version']}")
                print(f"  - Release: {update_info['release_name']}")
                print(f"  - Published: {update_info['published_at']}")
            else:
                print("✓ No update needed")
        else:
            print("✗ Update check failed")
        
        print("\n=== GitHub Integration Test Complete ===")
        return True
        
    except Exception as e:
        print(f"✗ Test failed with error: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_github_integration()
    sys.exit(0 if success else 1)
