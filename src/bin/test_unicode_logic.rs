//! Test the exact filter_contained_matches logic

use std::collections::HashSet;

fn main() {
    // Simulate the sorted matches
    // sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)

    // unicode_3: qstart=985, hilen=69, len=468, matcher=3 -> (985, -69, -468, 3)
    // unicode_40: qstart=985, hilen=18, len=134, matcher=2 -> (985, -18, -134, 2)
    // unicode_42: qstart=1127, hilen=56, len=341, matcher=2 -> (1127, -56, -341, 2)

    println!("=== SORTED ORDER ===");
    println!("1. unicode_3: (985, -69, -468, 3)");
    println!("2. unicode_40: (985, -18, -134, 2)");
    println!("3. unicode_42: (1127, -56, -341, 2)");

    println!("\n=== PYTHON FILTER LOGIC ===");
    println!("i=0, j=1: current=unicode_3, next=unicode_40");
    println!("  next.qend (1119) > current.qend (1468)? NO");
    println!("  current.qcontains(next)? unicode_3.qcontains(unicode_40)? YES");
    println!("  -> REMOVE unicode_40, continue (j stays 1)");

    println!("\ni=0, j=1: current=unicode_3, next=unicode_42");
    println!("  next.qend (1468) > current.qend (1468)? NO");
    println!("  current.qcontains(next)? unicode_3.qcontains(unicode_42)? NO (has gaps)");
    println!("  next.qcontains(current)? unicode_42.qcontains(unicode_3)? NO");
    println!("  -> j += 1 (j=2)");

    println!("\ni=0, j=2: j >= len(matches)? YES");
    println!("  -> break out of inner loop");
    println!("  -> i += 1 (i=1)");

    println!("\ni=1, i >= len(matches)-1? NO (matches now has 2 elements)");
    println!("  j = i+1 = 2, j < len(matches)? NO");
    println!("  -> break out of inner loop");
    println!("  -> i += 1 (i=2)");

    println!("\ni=2, i >= len(matches)-1? YES");
    println!("  -> exit outer loop");

    println!("\n=== EXPECTED RESULT ===");
    println!("Matches: [unicode_3, unicode_42]");
    println!("But Python gets: [unicode_40, unicode_42]");

    println!("\n=== CRITICAL INSIGHT ===");
    println!("The issue is that unicode_3 (sequence match, matcher=3) should NOT");
    println!("be able to discard unicode_40 (aho match, matcher=2) because AHO");
    println!("matches are higher quality than sequence matches!");
}
