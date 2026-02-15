//! Refinement and cleanup functions for detected copyright strings.
//!
//! After the parser produces raw detection text from parse tree nodes,
//! these functions clean up artifacts: strip junk prefixes/suffixes,
//! normalize whitespace, remove duplicate copyright words, strip
//! unbalanced parentheses, and filter out known junk patterns.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::candidates::strip_balanced_edge_parens;

// ─── Constant sets ───────────────────────────────────────────────────────────

/// Generic prefixes stripped from names (holders/authors).
const PREFIXES: &[&str] = &[
    "?",
    "??",
    "????",
    "(insert",
    "then",
    "current",
    "year)",
    "maintained",
    "by",
    "developed",
    "created",
    "written",
    "recoded",
    "coded",
    "modified",
    // Note: Python has 'maintained''created' (missing comma = concatenation).
    // We include both separately.
    "maintainedcreated",
    "$year",
    "year",
    "uref",
    "owner",
    "from",
    "and",
    "of",
    "to",
    "for",
    "or",
    "<p>",
];

/// Suffixes stripped from copyright strings.
static COPYRIGHTS_SUFFIXES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "copyright",
        ".",
        ",",
        "year",
        "parts",
        "any",
        "0",
        "1",
        "author",
        "all",
        "some",
        "and",
        "</p>",
        "is",
        "-",
        "distributed",
        "information",
        "credited",
    ]
    .into_iter()
    .collect()
});

/// Authors prefixes = PREFIXES ∪ author-specific words.
static AUTHORS_PREFIXES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s: HashSet<&str> = PREFIXES.iter().copied().collect();
    for w in &[
        "contributor",
        "contributors",
        "contributor(s)",
        "authors",
        "author",
        "authors'",
        "author:",
        "author(s)",
        "authored",
        "created",
        "author.",
        "author'",
        "authors,",
        "authorship",
        "maintainer",
        "co-maintainer",
        "or",
        "spdx-filecontributor",
        "</b>",
        "mailto:",
        "name'",
        "a",
        "moduleauthor",
        "\u{a9}", // ©
    ] {
        s.insert(w);
    }
    s
});

/// Authors junk — detected author strings that are false positives.
static AUTHORS_JUNK: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "james hacker.",
        "james random hacker.",
        "contributor. c. a",
        "grant the u.s. government and others",
        "james random hacker",
        "james hacker",
        "company",
        "contributing project",
        "its author",
        "gnomovision",
        "would",
        "may",
        "attributions",
        "the",
        "app id",
        "project",
        "previous lucene",
        "group",
        "the coordinator",
        "the owner",
        "a group",
        "sonatype nexus",
        "apache tomcat",
        "visual studio",
        "apache maven",
        "visual studio and visual studio",
        "work",
        "additional",
        "builder",
        "guice",
        "incorporated",
    ]
    .into_iter()
    .collect()
});

/// Prefix that triggers ignoring the author entirely.
const AUTHORS_JUNK_PREFIX: &str = "httpProxy";

static AUTHORS_JUNK_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let patterns = [
        r"(?i)\bpromote products derived from\b",
        r"(?i)\bendorse or promote\b",
        r"(?i)^the builder\b",
        r"(?i)^the line highlight\b",
        r"(?i)^the initial developer\b",
        r"(?i)^trademark\b",
        r"(?i)^time to time\b",
        r"(?i)^the group of people\b",
        r"(?i)^by,? or\b",
        r"(?i)^lucene commit\b",
        r"(?i)^group conversion\b",
        r"(?i)^grunt and npm\b",
        r"(?i)^bigscience\.\b",
        r"(?i)^ctnewmethod\b",
        r"(?i)\bplugins?\. fixes\b",
        r"(?i)\bnormalized to upper\b",
        r"(?i)\benhancing and supporting\b",
        r"(?i)\band to credit the\b",
        r"(?i)^other promise\b",
        r"(?i)^record factory\b",
        r"(?i)^the object\b",
        r"(?i)^the owner,?\b",
        r"(?i)^the job$",
        r"(?i)^the ietf\b",
        r"(?i)^manually\b",
        r"(?i)^register\b",
        r"(?i)^communication sent\b",
        r"(?i)^developers tom\b",
        r"(?i)^donald becker$",
        r"(?i)^ext4\.\b",
        r"(?i)\bmore documentation\b",
        r"(?i)\breturn enum\b",
        r"(?i)\breturn u\d",
        r"(?i)\bmore details of status\b",
        r"(?i)\bu64$",
        r"(?i)^\d+\.\d+\s+\d+-\w+-\d+\s+fix\b",
        r"(?i)\bbut not limited to communication\b",
        r"(?i)\bfor have helping\b",
        r"(?i)\bunit of \d+mb\b",
        r"(?i)\bfor the openssl project\b",
        r"(?i)\bwith participation of the open\b",
        r"(?i)\bfurthermore\b",
        r"(?i)\bits cell\. we\b",
        r"(?i)\b@version \$id\b",
        r"(?i)\bsymbols viewer\b",
        r"(?i)\bfinal specification itself\b",
        r"(?i)\bfor each of the audio\b",
        r"(?i)\bfrom start to end\b",
        r"(?i)\boperator to\b",
        r"(?i)^programming with objects\b",
        r"(?i)^grateful to\b",
        r"(?i)^would also like to thank\b",
        r"(?i)^would like to thank\b",
        r"(?i)^intellij idea$",
        r"(?i)^date modified$",
        r"(?i)^date header id name\b",
        r"(?i)^technical committee$",
        r"(?i)^users of the program$",
        r"(?i)^should not be interpreted\b",
        r"(?i)^philip$",
        r"(?i)^john$",
        r"(?i)^arnaldo carvalho de melo\b",
        r"(?i)^works devices national\b",
        r"(?i)\band its \d+\.\s*neither\b",
        r"(?i)\band its effective immediately\b",
        r"(?i)\band its neither the\b",
        r"(?i)\blastmod\b.*\bstream.*\bregisterfield\b",
        r"(?i)^hillion$",
    ];
    patterns.iter().filter_map(|p| Regex::new(p).ok()).collect()
});

fn is_junk_author(s: &str) -> bool {
    AUTHORS_JUNK_PATTERNS.iter().any(|re| re.is_match(s))
}

/// Holders prefixes = PREFIXES ∪ holder-specific words.
static HOLDERS_PREFIXES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s: HashSet<&str> = PREFIXES.iter().copied().collect();
    for w in &[
        "-",
        "a",
        "<a",
        "href",
        "ou",
        "portions",
        "portion",
        "notice",
        "holders",
        "holder",
        "property",
        "parts",
        "part",
        "at",
        "cppyright",
        "assemblycopyright",
        "c",
        "works",
        "present",
        "right",
        "rights",
        "reserved",
        "held",
        "is",
        "(x)",
        "later",
        "$",
        "current.year",
        "\u{a9}", // ©
        "author",
        "authors",
    ] {
        s.insert(w);
    }
    s
});

/// Holders prefixes including "all" (used when "reserved" is in the string).
static HOLDERS_PREFIXES_WITH_ALL: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HOLDERS_PREFIXES.clone();
    s.insert("all");
    s
});

/// Suffixes stripped from holder strings.
static HOLDERS_SUFFIXES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "http",
        "and",
        "email",
        "licensing@",
        "(minizip)",
        "website",
        "(c)",
        "<http",
        "/>",
        ".",
        ",",
        "year",
        "some",
        "all",
        "right",
        "rights",
        "reserved",
        "reserved.",
        "href",
        "c",
        "a",
        "</p>",
        "or",
        "taken",
        "from",
        "is",
        "-",
        "distributed",
        "information",
        "credited",
    ]
    .into_iter()
    .collect()
});

/// Holders junk — detected holder strings that are false positives.
static HOLDERS_JUNK: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "a href",
        "property",
        "licensing@",
        "c",
        "works",
        "http",
        "the",
        "are",
        "?",
        "cppyright",
        "parts",
        "disclaimed",
        "or",
        "<holders>",
        "author",
        // License boilerplate false positives
        "holders",
        "holder",
        "holder,",
        "and/or",
        "if",
        "grant",
        "does",
        "has",
        "each",
        "also",
        "in",
        "simply",
        "other",
        "shall",
        "said",
        "who",
        "your",
        "their",
        "ensure",
        "allow",
        "terms",
        "information",
        "contributors",
        "indemnification",
        "license",
        "claimed",
        "but",
        "agrees",
        "patent",
        "owner",
        "yyyy",
        "expressly",
        "stating",
        "enforce",
        "d",
        "ss",
        // Additional single-word junk
        "given",
        "may",
        "every",
        "no",
        "good",
        "row",
        "logo",
        "flag",
        "updated",
        "law",
        "england",
        "tm",
        "pgp",
        "distributed",
        "as",
        "null",
        "psy",
        "object",
        // Short gibberish from binary data
        "ga",
        "ka",
        "aa",
        "qa",
        "yx",
        "ac",
        "gn",
        "cb",
        "ib",
        "qb",
        "py",
        "pu",
        "ce",
        "nmd",
        "a1",
        "deg",
        "gnu",
        "with",
        "yy",
        "c/",
        "messages",
        "licenses",
        "not limited",
        "charge",
        "case 2",
        "dot",
        // C function/macro names from ICS false positives
        "width",
        "len",
        "do",
        "date",
        "year",
        "note",
        "update",
        "info",
        "notices",
        "duplicated",
        "register",
        // C identifier/keyword false positives from ICS
        "isascii",
        "iscntrl",
        "isprint",
        "isdigit",
        "isalpha",
        "toupper",
        "yyunput",
        "ambiguous",
        "indir",
        "notive",
        "strict",
        "decoded",
        "unsigned",
        // Short numbers/tokens from code
        "0 1",
        "8",
        "9",
        "16",
        "24",
        "4",
        // More boilerplate/legal words
        "notices all the files",
        "may not be removed or altered",
        "duplicated in",
    ]
    .into_iter()
    .collect()
});

/// Junk copyright regex patterns (compiled once).
static COPYRIGHTS_JUNK_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let patterns = [
        r"(?i)^copyright \(c\)$",
        r"(?i)^\(c\) by$",
        r"(?i)\(c\) [a-zA-Z][a-z] \(c\)",
        r"(?i)^copyright holder or simply",
        r"(?i)^copyright notice\.",
        r"(?i)^copyright of uc berkeley's berkeley software distribution",
        r"(?i)^and/or the universal copyright convention",
        r"(?i)^attn copyright",
        r"(?i)^\(c\)$",
        r"(?i)^c$",
        r"(?i)^\(c\) any recipient$",
        r"(?i)^\(c\) as$",
        r"(?i)^\(c\),? \(c\)$",
        r"(?i)^\(c\) cockroach enterprise",
        r"(?i)^\(c\) each recipient$",
        r"(?i)^\(c\) forums$",
        r"(?i)^\(c\) if you",
        r"(?i)^\(c\) individual use",
        r"(?i)^code copyright grant",
        r"(?i)^copyright and license, contributing",
        r"(?i)^copyright as is group",
        r"(?i)^copyright \(c\) , and others",
        r"(?i)^copyright-check writable-files m4-check author_mark_check",
        r"(?i)^copyright \(c\) <holders>",
        r"(?i)^copyright copyright and",
        r"(?i)^copyright \(c\) year$",
        r"(?i)^copyright \(c\) year your",
        r"(?i)^copyright, designs and patents",
        r"(?i)copyright \d+ m\. y\.( name)?",
        r"(?i)^copyrighte?d? (by)?$",
        r"(?i)^copyrighted by its$",
        r"(?i)^copyrighted by their authors",
        r"(?i)^copyrighted material, only this license",
        r"(?i)^copyright for a new language",
        r"(?i)^copyright from license",
        r"(?i)^copyright help center",
        r"(?i)^copyright holder and contributors?\.?$",
        r"(?i)^copyright-holder and its contributors$",
        r"(?i)^copyright holder has",
        r"(?i)^copyright holder means",
        r"(?i)^copyright holder who",
        r"(?i)^copyright holder nor",
        r"(?i)^copyright holder,? or",
        r"(?i)^copyright holders and contribut",
        r"(?i)^copyright holder's",
        r"(?i)^copyright holder\(s\) or the author\(s\)",
        r"(?i)^copyright including",
        r"(?i)^copyright in section",
        r"(?i)^copyright john wiley & sons, inc\. year",
        r"(?i)^copyright l?gpl group",
        r"(?i)^copyright, license, and",
        r"(?i)^copyright merged arm",
        r"(?i)^copyright neither",
        r"(?i)^copyright notices, authorship",
        r"(?i)^copyright not limited",
        r"(?i)^copyright owner or",
        r"(?i)^copyright redistributions",
        r"(?i)^copyright the project$",
        r"(?i)^copyright\.? united states$",
        r"(?i)^\(c\) software activation",
        r"(?i)^\(c\) source code",
        r"(?i)^full copyright statement",
        r"(?i)^universal copyright convention",
        r"(?i)^u\.s\. copyright act",
        r"(?i)^\(c\) Object c$",
        r"(?i)^copyright headers?",
        r"(?i)Copyright \(c\) 2021 Dot",
        r"(?i)^\(c\) \(c\) B$",
        r"(?i)^\(c\) group$",
        r"(?i)^\(c\) \(c\) A$",
        r"(?i)^\(c\) the\b",
        r"(?i)^\(c\) if\b",
        r"(?i)^\(c\) for\b",
        r"(?i)^\(c\) to\b",
        r"(?i)^\(c\) one\b",
        r"(?i)^\(c\) all\b",
        r"(?i)^\(c\) allow\b",
        r"(?i)^\(c\) ensure\b",
        r"(?i)^\(c\) permit\b",
        r"(?i)^\(c\) delete\b",
        r"(?i)^\(c\) return\b",
        r"(?i)^\(c\) flag\b",
        r"(?i)^\(c\) charge\b",
        r"(?i)^\(c\) automatically\b",
        r"(?i)^\(c\) completely\b",
        r"(?i)^\(c\) terminate\b",
        r"(?i)^\(c\) suspend\b",
        r"(?i)^\(c\) material\b",
        r"(?i)^\(c\) indemnification\b",
        r"(?i)^\(c\) england\b",
        r"(?i)^\(c\) a$",
        r"(?i)^\(c\) b$",
        r"(?i)^\(c\) c$",
        r"(?i)^\(c\) s$",
        r"(?i)^\(c\) u$",
        r"(?i)^\(c\) this\.",
        r"(?i)^\(c\) nat\d",
        r"(?i)^\(c\) ss+y?$",
        r"(?i)^\(c\) objc",
        r"(?i)^\(c\) \.year",
        r"(?i)^\(c\) case\b",
        r"(?i)^\(c\) offer\b",
        r"(?i)^\(c\) compute\b",
        r"(?i)^\(c\) there\b",
        r"(?i)^\(c\) c printf\b",
        r"(?i)^\(c\) -\d",
        r"(?i)^\(c\) ac$",
        r"(?i)^\(c\) eu$",
        r"(?i)^\(c\) continue\b",
        r"(?i)^\(c\) component\b",
        r"(?i)^\(c\) ext\.",
        r"(?i)^\(c\) assert\.",
        r"(?i)^\(c\) ,\(d\)",
        r"(?i)^copyright notice\b",
        r"(?i)^copyright holders? be\b",
        r"(?i)^copyright holders? and/?or\b",
        r"(?i)^copyright holders?$",
        r"(?i)^copyright holders? shall\b",
        r"(?i)^copyright holder saying\b",
        r"(?i)^copyright holders of\b",
        r"(?i)^copyright holder,$",
        r"(?i)^copyright holder notifies\b",
        r"(?i)^copyright holder is reinstated\b",
        r"(?i)^copyright holder fails\b",
        r"(?i)^copyright holders, but\b",
        r"(?i)^copyright holders, or\b",
        r"(?i)^copyright holders, authors\b",
        r"(?i)^copyright holder\. ",
        r"(?i)^copyright holder, author\b",
        r"(?i)^copyright holders? disclaim\b",
        r"(?i)^copyright and has\b",
        r"(?i)^copyright and trademark\b",
        r"(?i)^copyright and other proprietary\b",
        r"(?i)^copyright in the\b",
        r"(?i)^copyright in and\b",
        r"(?i)^copyright the software\b",
        r"(?i)^copyright info for\b",
        r"(?i)^copyright grant\b",
        r"(?i)^copyright terms\b",
        r"(?i)^copyright does\b",
        r"(?i)^copyright unless\b",
        r"(?i)^copyright also\b",
        r"(?i)^copyright are\b",
        r"(?i)^copyright line\b",
        r"(?i)^copyright resulting\b",
        r"(?i)^copyright treaty\b",
        r"(?i)^copyright rights\b",
        r"(?i)^copyright appears?\b",
        r"(?i)^copyright years? updated\b",
        r"(?i)^copyright license\b",
        r"(?i)^copyright copyright\b",
        r"(?i)^copyrights covering\b",
        r"(?i)^copyrights for the\b",
        r"(?i)^copyright for the\b",
        r"(?i)^copyright symbol\b",
        r"(?i)^copyright claim\b",
        r"(?i)^copyright interest\b",
        r"(?i)^copyright shall\b",
        r"(?i)^copyright statement\b",
        r"(?i)^copyright disclaimer\b",
        r"(?i)^copyright permission\b",
        r"(?i)^copyright protection\b",
        r"(?i)^copyright owner\b",
        r"(?i)^copyright yyyy\b",
        r"(?i)^copyright exceptions\b",
        r"(?i)^copyright or patent\b",
        r"(?i)^copyright is claimed\b",
        r"(?i)^copyright messages\b",
        r"(?i)^copyright information\b",
        r"(?i)^copyright at the\b",
        r"(?i)^copyright claimed\b",
        r"(?i)^copyright law\b",
        r"(?i)^copyright page\b",
        r"(?i)^copyright holders? or\b",
        r"(?i)^copyrighted works\b",
        r"(?i)^copyrighted material outside\b",
        r"(?i)^copyright holder as a result\b",
        r"(?i)^copyright holder explicitly\b",
        r"(?i)^copyright holder collectively\b",
        r"(?i)^copyright holder stating\b",
        r"(?i)^copyright holder to enforce\b",
        r"(?i)^copyright holder expressly\b",
        r"(?i)^copyright holder maintains\b",
        r"(?i)^copyright holder may\b",
        r"(?i)^copyright holder is whoever\b",
        r"(?i)^copyright holder, and\b",
        r"(?i)^copyright holder, but\b",
        r"(?i)^copyright holder and seek\b",
        r"(?i)^copyright holder of\b",
        r"(?i)^copyright of\b",
        r"(?i)^copyright or\b",
        r"(?i)^copyright is held by\b",
        r"(?i)^copyright as specified\b",
        r"(?i)^copyrights and patent\b",
        r"(?i)^copyright holder provides\b",
        r"(?i)^copyright holder agrees\b",
        r"(?i)^copyright holder and current maintainer\b",
        r"(?i)^copyright holder,?\s*referring\b",
        r"(?i)\bmaintainer referring to the person\b",
        r"(?i)\bexplicitly and prominently states\b",
        r"(?i)\bm\. y\. name\b",
        r"(?i)^copyrights are property of\b",
        r"(?i)^copyright holder,? we do not list\b",
        r"(?i)^copyright and no-warranty notice\b",
        r"(?i)^copyright pages? of volumes?\b",
        r"(?i)^copyright as is\b",
        r"(?i)^copyright its (contributors|licensors|respective)\b",
        r"(?i)^copyright owned\b",
        r"(?i)^copyright attr\b",
        r"(?i)^copyright content\b",
        r"(?i)^copyright a href\b",
        r"(?i)^copyright designation\b",
        r"(?i)^copyright infringement\b",
        r"(?i)^copyright General Public\b",
        r"(?i)^copyright owners\b",
        r"(?i)^copyright and as\b",
        r"(?i)^copyright applies\b",
        r"(?i)^copyrights of all\b",
        r"(?i)^copyright As I\b",
        r"(?i)^copyright by The Regents\b",
        r"(?i)^copyright by other\b",
        r"(?i)^copyrighted by C\.\b",
        r"(?i)^copyright note\b",
        r"(?i)^copyright clause\b",
        r"(?i)^copyright message\b",
        r"(?i)^copyright below\b",
        r"(?i)^copyright is below\b",
        r"(?i)^copyright date\b",
        r"(?i)^copyright year$",
        r"(?i)^copyright notive\b",
        r"(?i)^copyright inside\b",
        r"(?i)^copyright match\b",
        r"(?i)^copyright notices\b",
        r"(?i)^copyright GNU\b",
        r"(?i)^COPYRIGHT AS PER\b",
        r"(?i)^Copyright and Related Rights\b",
        r"(?i)^copyright by Section\b",
        r"(?i)^Copyright The GNOME\b",
        r"(?i)^Copyright The$",
        r"(?i)^Copyright notices\b",
        r"(?i)^copyright to$",
        r"(?i)^copyrights in$",
        r"(?i)^copyright to the\b",
        r"(?i)^copyrighted \(with\b",
        r"(?i)^\(Copyright notice\)",
        r"(?i)^COPYRIGHT HOLDER ALLOWS\b",
        r"(?i)^copyright holders?,? disclaims?\b",
        r"(?i)\bwe do not list the\b",
        r"(?i)\bno-warranty notice unaltered\b",
        r"(?i)\bprovides the program as\b",
        r"(?i)\breferring to the person\b",
        r"(?i)\bderivatives of$",
        r"(?i)^copyright in$",
        r"(?i)^copyright and other$",
        r"(?i)\bline and a pointer to where\b",
        r"(?i)\binterest in the program\b",
        r"(?i)\binterest in the library\b",
        r"(?i)\bhas no obligation to provide maintenance\b",
        r"(?i)^be liable to\b",
        r"(?i)\bthe respective terms and conditions\b",
        r"(?i)\bthe terms and conditions of the copyright\b",
        r"(?i)\bwho places the library\b",
        r"(?i)\bthe library among them\b",
        r"(?i)\bdisclaimer for the library\b",
        r"(?i)\bprofile authors\s+@remark",
        r"(?i)\banybody can make use of my programs\b",
        r"(?i)\bof computers and typesetting\b",
        r"(?i)^copyright the library,?$",
        r"(?i)^\(c\) endif$",
        r"(?i)^endif$",
        r"(?i)^\(c\) \?$",
        r"(?i)^\(c\) [a-z]$",
        r"(?i)^[a-z]$",
        r"(?i)^\(c\) [a-z] [a-z]$",
        r"(?i)^[a-z] [a-z]$",
        r"(?i)^\(c\) ISLOWER$",
        r"(?i)^ISLOWER$",
        r"(?i)^\(c\) - [a-z]$",
        r"(?i)^0$",
        r"(?i)^\(c\) 0$",
        r"(?i)^Copyright \(c\) \d{4}$",
        r"(?i)^Copyright \d{4}$",
        r"(?i)^\(c\) \d{4}$",
        r"(?i)^year\(\d{4}\)\.format\b",
        r"(?i)^SSY$",
        r"(?i)^Object$",
        // "Copyright Holder as/to/the" boilerplate
        r"(?i)^copyright holder as specified\b",
        r"(?i)^copyright holder to\b",
        r"(?i)^copyright holder,? the\b",
        r"(?i)^copyrights as noted\b",
        r"(?i)^COPYRIGHT DOCUMENTATION\b",
        r"(?i)^COPYRIGHT STATEMENTS\b",
        r"(?i)^copyright and other intellectual\b",
        r"(?i)^copyright treaties\b",
        // (c) followed by code-like constructs
        r"(?i)^\(c\) [\!\?&\|\.;:,\+\-\*/<>=]",
        r"(?i)^\(c\) [\w]+\.\w+\(",
        r"(?i)^\(c\) [\w]+\[",
        r"(?i)^\(c\) &&",
        r"(?i)^\(c\) \|\|",
        r"(?i)^\(c\) [\w]+\?",
        r"(?i)^\(c\) [\w]+\.[\w]+\.",
        r"(?i)^\(c\) [\w]+\([\w,]+\)",
        // (c) followed by short gibberish (1-3 mixed-case chars) from binary data
        r"^\(c\) [A-Z][a-z]{1,2}$",
        // (c) followed by "Unknown" (binary/PDF artifacts)
        r"(?i)^\(c\) Unknown\b",
        // (c) followed by binary/garbled data patterns
        r"^\(c\) [^\x20-\x7E]",
        r"^\(c\) [\x00-\x1F]",
        r"^\(c\) [A-Z][a-z]+ d [A-Z][a-z]+$",
        r"^\(c\) [A-Z]{2,}[0-9]",
        r"^\(c\) [a-z]{1,3}$",
        // (c) followed by C code patterns
        r"(?i)^\(c\) c -",
        r"(?i)^\(c\) c TOUPPER",
        r"(?i)^\(c\) isascii",
        r"(?i)^\(c\) isupper",
        r"(?i)^\(c\) isdigit",
        r"(?i)^\(c\) isalnum",
        r"(?i)^\(c\) isalpha",
        r"(?i)^\(c\) isspace",
        r"(?i)^\(c\) iscntrl",
        r"(?i)^\(c\) isprint",
        r"(?i)^\(c\) ifdef",
        r"(?i)^\(c\) undef\b",
        r"(?i)^\(c\) endif\b",
        r"(?i)^\(c\) sgn\b",
        r"(?i)^\(c\) dst",
        r"(?i)^\(c\) ptr\b",
        r"(?i)^\(c\) slen\b",
        r"(?i)^\(c\) len\b",
        r"(?i)^\(c\) do$",
        r"(?i)^\(c\) uint",
        r"(?i)^\(c\) gunichar\b",
        r"(?i)^\(c\) TRUE FALSE",
        r"(?i)^\(c\) yyunput\b",
        r"(?i)^\(c\) yylval\b",
        r"(?i)^\(c\) ungetc\b",
        r"(?i)^\(c\) 0x[0-9a-fA-F]",
        r"(?i)^\(c\) \(\(unsigned",
        r"(?i)^\(c\) \(int\)",
        r"(?i)^\(c\) \(uint",
        r"(?i)^\(c\) \(s\)",
        r"(?i)^\(c\) \d+ \(\(", // "(c) 16 ((d) 24)"
        r"(?i)^\(c\) \d+ &",    // "(c) 6 (trail&0x3f)"
        r"(?i)^\(c\) strict\b",
        r"(?i)^\(c\) width\b",
        r"(?i)^\(c\) arg\b",
        r"(?i)^\(c\) cindex\b",
        r"(?i)^\(c\) foot-",
        r"(?i)^\(c\) put\b",
        r"(?i)^\(c\) DEBUGP\b",
        r"(?i)^\(c\) Chain\b",
        r"(?i)^\(c\) Only\b",
        r"(?i)^\(c\) Walked\b",
        r"(?i)^\(c\) Construct\b",
        r"(?i)^\(c\) p can\b",
        r"(?i)^\(c\) c\.warn\b",
        r"(?i)^\(c\) b\.status\b",
        r"(?i)^\(c\) table\.set\b",
        r"(?i)^\(c\) in$",
        r"(?i)^\(c\) macro\b",
        r"(?i)^\(c\) decoded\b",
        r"(?i)^\(c\) IP_VS",
        r"(?i)^\(c\) Like\b",
        r"(?i)^\(c\) Page\b",
        r"(?i)^\(c\) WITH\b",
        r"(?i)^\(c\) \(1\b",
        r"(?i)^\(c\) \(2\)",
        r"(?i)^\(c\) \(MON\b",
        r"(?i)^\(c\) M this\b",
        r"(?i)^\(c\) \(0,",
        // (c) followed by PDF/PostScript artifacts
        r"(?i)^\(c\) Tj\b",
        r"(?i)^\(c\) ET\b",
        r"(?i)^\(c\) Registered$",
        // (c) followed by garbled/encoded text
        r"^\(c\) uL",
        r"^\(c\) [¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ]",
        r"^\(c\) .*ÿÿÿ",
        r"^\(c\) .*°°°",
        // (c) followed by license/legal boilerplate
        r"(?i)^\(c\) Inclusion\b",
        r"(?i)^\(c\) Whenever\b",
        r"(?i)^\(c\) Customer",
        r"(?i)^\(c\) Splunk\b",
        r"(?i)^\(c\) No$",
        r"(?i)^\(c\) CockroachDB\b",
        r"(?i)^\(c\) Custom Nessus\b",
        r"(?i)^\(c\) Products\.",
        r"(?i)^\(c\) \u{201c}", // left double quotation mark
        // (c) followed by number-only patterns (not years)
        r"^\(c\) \d{1,2}$",
        r"^\(c\) \d+ \d+ y\b",
        // (c) followed by PostScript/font data
        r"(?i)^\(c\) SS'",
        r"(?i)^\(c\) PSPSY",
        r"(?i)^\(c\) PSY$",
        r"(?i)^\(c\) a! ",
        r"(?i)^\(c\) aae\b",
        r"(?i)^\(c\) \(r\)",
        r"(?i)^\(c\) D'O\b",
        r"(?i)^\(c\) AT r'b",
        r"(?i)^\(c\) C,BLACK",
        r"(?i)^\(c\) hUja\b",
        r"(?i)^\(c\) NULL$",
        r"(?i)^\(c\) cc\.fr",
        r"(?i)^\(c\) Oo2\b",
        r"(?i)^\(c\) UOSSOO",
        r"(?i)^\(c\) q ltd",
        r"(?i)^\(c\) zbar",
        r"(?i)^\(c\) distributed$",
        r"(?i)^\(c\) \(tm\)",
        r"(?i)^\(c\) ,\s*,",
        r"(?i)^\(c\) notice\b",
        r"(?i)^\(c\) create\b",
        r"(?i)^\(c\) do not\b",
        r"(?i)^\(c\) give\b",
        r"(?i)^copyright logo\b",
        r"(?i)^copyright targetpath\b",
        r"(?i)^copyright \(xmlns\b",
        r"(?i)^copyright its authors\b",
        r"(?i)^\(copyright\s*\)\b",
        r"(?i)^copyright the product\b",
        r"(?i)^copyright year\b.*\bfor\b",
        r"(?i)^copyrights? and licenses\b",
        r"(?i)^copyright applied to\b",
        r"(?i)^copyrighted material,\b",
        r"(?i)^copyright is\b",
        r"(?i)^\(c\) of the\b",
        r"(?i)^\(c\) other$",
        r"(?i)^\(c\) dates of\b",
        r"(?i)^\(c\) improved syntax\b",
        r"(?i)^\(c\),?\s*,",
        r"(?i)^\(c\),?\s*group\b",
        r"(?i)^\(c\),?\s*count\b",
        r"(?i)^\(c\),?\s*b\s",
        r"(?i)^\(c\),?\s*c$",
        r"(?i)^copyright act\b",
        r"(?i)^copyright for$",
        r"(?i)^copyright holder for the\b",
        r"(?i)^copyright man page\b",
        r"(?i)^copyright s status\b",
        r"(?i)^copyright and things like\b",
        r"(?i)^copyrights cover\b",
        r"(?i)^copyrights in the original\b",
        r"(?i)^copyrights in the portions\b",
        r"(?i)^copyrighted$",
        r"(?i)^copyright tue\b",
        r"(?i)^copyright sign\b",
        r"(?i)^c.opylefted\b",
        r"(?i)^i\.\s*\(c\)\b",
        r"(?i)^u1e\s*\(c\)\b",
        r"(?i)^xz\b.*\(c\)\b",
        r"^\(c\) [A-Z]{3,}[a-z]{1,3}$",
        r"^\(c\) [A-Z][a-z][A-Z][a-z]",
        r"^\(c\) [A-Z]{2}[a-z][A-Z]",
        r"^\(c\) [A-Z][A-Z][a-z][a-z][a-z]?[A-Z]",
        r"(?i)^copyright info have been\b",
        r"(?i)^\(copyright\s*\)\s*gnu general\b",
        r"(?i)^\(copyright\b.*\bvoltagefactor\b",
        r"(?i)^\(copyright unasserted\)\b",
        r"(?i)^copyright the lavantech\b",
        r"(?i)^copyright year united states\b",
        r"(?i)^copyright 1991-\d+ imatix\b.*\bwith exception\b",
        r"(?i)^\(c\) io\\0",
        r"(?i)^\(c\) ecfieldelement\b",
        r"(?i)^\(c\) distributed\b",
        r"(?i)^\(c\) yyyy\b",
        r"(?i)^\(c\) rebel\b",
        r"(?i)^\(c\) metastuff\b",
        r"(?i)^\(c\) mihai\b",
        r"(?i)^\(c\) linux foundation\b.*\bunified\b",
        r"(?i)^\(c\) helge deller\b.*\bcopyright\b",
        r"(?i)^\(c\) hewlett-packard company$",
        r"(?i)^copyright \(c\) david j\. bradshaw$",
        r"(?i)^copyright \(c\) tim ruffles$",
        r"(?i)^copyright \(c\) gias kay lee$",
        r"(?i)^copyright \(c\) xerox corporation$",
        r"(?i)^copyright -+\s*copyright\b",
        r"(?i)^copyright \u{fffd}",
        r"(?i)^\u{fffd}\d+-\d+\b",
        r"(?i)^nexb and others\b",
        r"(?i)^copyright \u{a9}\d",
        r"(?i)^copyrighted material,? only\b",
        r"(?i)^copyrights of the\b",
        r"(?i)^\(c\) p b i n do$",
        r"(?i)^\(c\) 2004-2009 pudn\.com\b",
        r"(?i)^.{1,5}\s*\(c\)\s*.{1,5}$",
        r"(?i)^swfobject\b.*\bcopyright\b",
        r"(?i)^the the oscar\b",
        r"(?i)^\(c\) 2004-2010$",
        r"(?i)^\(c\) 1997 m\. kirkwood converted\b",
        r"(?i)^\(c\) 1998 red hat tcp\b",
        r"(?i)^\(c\) 1999 david airlie\b.*\bbugfixes\b",
        r"(?i)^\(c\) 1998-2002 by heiko eissfeldt\b",
        r"(?i)^\(c\) 2001 dave jones\b",
        r"(?i)^\(c\) 2003-2004 paul clements\b",
        r"(?i)^\(c\) 2014-\$$",
        r"(?i)^copyright 2014-\$$",
        r"(?i)^copyright 2010 ben dooks fluff\b",
        r"(?i)^\(c\)\s*(indir|then|unacceptable)\b",
        r"(?i)^\(c\) c arg\b",
        r"(?i)^\(c\) @ ?(symrec|ungetc|yylval)\b",
        r"(?i)^\(c\) \(the parens\b",
        r"(?i)^\(c\) s-\d",
        r"(?i)^\(c\) register\b",
        r"(?i)^\(c\) Mouse Wheel\b",
        r"(?i)^copyright info$",
        r"(?i)^copyright for a\b",
        r"(?i)^COPYRIGHT HOLDERS AS\b",
        r"(?i)@remark Read",
        r"(?i)\bContact <\w+@\w+",
        r"(?i)^\d{1,2}$",
        r"(?i)^\(c\) yyunput\b",
        r"(?i)^\(c\) yylval\b",
        r"(?i)^IsLower\s*\(c\)\s*IsDigit\b",
        r"(?i)^copyright \d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}\b",
        r"(?i)^Copyright \d{4}-\d{4}$",
        r"(?i)^Copyright \(c\) \d{4}-\d{4}$",
        r"(?i)^Copyright \(c\) \d{4} Contributors$",
        r"(?i)^ds Status works\b",
        r"(?i)^Copyright \(c\) The team$",
        r"(?i)^holder\.\s*AS\b",
        r"(?i)^as\(c,\s*field\b",
        r"(?i)^skb\.\s*The buffer\b",
        r"(?i)^partial mlock\b",
        // (c) followed by C variable/type patterns
        r"(?i)^\(c\) (unsigned|int|char|void|long|short|float|double|static|struct)\b",
        r"(?i)^\(c\) (classify|ctable|cvPoint|fWidth|macroptr|MAGIC)\b",
        r"(?i)^\(c\) (letters|ok letters)\b",
        r"(?i)^\(c\) (res|ret|run|save|sizeof|temp)\b",
        r"(?i)^\(c\) (flags|buffer|buflen)\b",
        r"(?i)^\(c\) (pr |prec |printf )\b",
        r"(?i)^\(c\) (Accumulate|Bit8u|Returns the|SkReplicate)\b",
        r"(?i)^\(c\) (asm|xlp|we copy)\b",
        r"(?i)^\(c\) (d \d|num \d|mat \d)\b",
        r"(?i)^\(c\) do (prec|while)\b",
        r"(?i)^\(c\) etc\b",
        r"(?i)^\(c\) Finn Thain\b.*\bCopying\b",
        r"(?i)^\(c\) Kasım\b",
        r"(?i)^\(c\) z \?$",
        r"(?i)^\(c\) Z \?$",
        r"(?i)^\(c\) \d+ \+0x",
        r"(?i)^\(c\) \d+ static\b",
        r"(?i)^\(c\) 0-9",
        r"(?i)^\(c\) 122$",
        r"(?i)^\(c\) \d+ endif\b",
        r"(?i)^\(c\) \(c&",
        r"(?i)^\(c\) \(cp\)",
        r"(?i)^\(c\) \( cp\)",
        r"(?i)^\(c\) \(l\)",
        r"(?i)^\(c\) \(out\.\b",
        r"(?i)^\(c\) \(run\)",
        r"(?i)^\(c\) \(scale\b",
        r"(?i)^\(c\) \^ \(",
        r"(?i)^\(c\) \(DBus",
        r"(?i)^\(c\) c c c\b",
        r"(?i)^\(c\) c toascii\b",
        r"(?i)^\(c\) c tolower\b",
        r"(?i)^\(c\) c \(qbuf\b",
        r"(?i)^\(c\) c / endif\b",
        r"(?i)^\(c\) c \^ 0x",
        r"(?i)^\(c\) c 03o\b",
        r"(?i)^\(c\) c 0x\d",
        r"(?i)^\(c\) this-\b",
        r"(?i)^\(c\) putchar\b",
        // (c) followed by year + trailing junk
        r"(?i)^\(c\) \d{4}(-\d{4})? Jean-loup Gailly\b.*\b(END|VALUE)\b",
        r"(?i)^\(c\) \d{4}(-\d{4})? Julian Seward\b.*\btitle\b",
        r"(?i)^\(c\) \d{4} Paul Rusty Russell\b.*\bPlaced\b",
        r"(?i)^\(c\) \d{4} Dan Potter\b.*\bmodify\b",
        r"(?i)^\(c\) \d{4} Red Hat\.\s*GPLd\b",
        r"(?i)^\(c\) \d{4}-\d{4}$",
        r"(?i)^\(c\) \d{4} Andreas Gruenbacher\b.*\bgruenbacher@\b",
        r"(?i)^\(c\) \d{4},?\s*\d{4},?\s*\d{4} Thomas Vander Stichele\b",
        r"(?i)^\(c\) \d{4} Adam Nielsen\b.*\bniel?sen@\b",
        r"(?i)^\(c\) \d+ \(trail",
        r"(?i)^\(c\) 4\+\(r\)",
        // copyright followed by non-copyright text
        r"(?i)^copyright :G2P\b",
        r"(?i)^copyright \d+ trademark\b",
        r"(?i)^copyright 60$",
        r"(?i)^copyright ACM and IEEE\b",
        r"(?i)^copyright and placed into\b",
        r"(?i)^copyright and to distribute\b",
        r"(?i)^copyright as follows\b",
        r"(?i)^copyright definedummyword\b",
        r"(?i)^copyright FILE\b",
        r"(?i)^copyright info to be\b",
        r"(?i)^copyright mea-\b",
        r"(?i)^copyright meta-\b",
        r"(?i)^copyright others$",
        r"(?i)^copyright problem,?\b",
        r"(?i)^copyright SGI\b",
        r"(?i)^copyright to help\b",
        r"(?i)^copyright year to\b",
        r"(?i)^copyrighted - provided\b",
        r"(?i)^copyrighted by the following\b",
        r"(?i)^copyrighted work\b",
        r"(?i)^copyrights apply\b",
        r"(?i)^copyrights to use\b",
        // Non-copyright holder-like strings that are false positives
        r"(?i)^count count\b",
        r"(?i)^const char\b",
        r"(?i)^int\s",
        r"(?i)^int$",
        r"(?i)^lack of warranty\b",
        r"(?i)^macro for checking\b",
        r"(?i)^mat \d\b",
        r"(?i)^MD5Update\b",
        r"(?i)^message$",
        r"(?i)^Nuance Communications,? but\b",
        r"(?i)^NULL,? \d",
        r"(?i)^placed into PD\b",
        r"(?i)^preserved in its entirety\b",
        r"(?i)^Protocol Engineering Lab\b",
        r"(?i)^ptr$",
        r"(?i)^Regents of the University\b.*\bBerkeley Software\b",
        r"(?i)^res$",
        r"(?i)^ret$",
        r"(?i)^run$",
        r"(?i)^sgn$",
        r"(?i)^sizeof$",
        r"(?i)^SIGN\(b\)",
        r"(?i)^strict forbid\b",
        r"(?i)^terms and conditions$",
        r"(?i)^toascii$",
        r"(?i)^tolower$",
        r"(?i)^trademark acute\b",
        r"(?i)^TRADEMARK \d+NOTICES\b",
        r"(?i)^true$",
        r"(?i)^unacceptable$",
        r"(?i)^unsigned\s+(char|int|long|short|b|g|r|sb|sg)\b",
        r"(?i)^we copy data\b",
        r"(?i)^work$",
        r"(?i)^wide$",
        r"(?i)^joint with$",
        r"^others$",
        r"(?i)^symbol,? for example\b",
        r"(?i)^the shared library will be\b",
        r"(?i)^SkReplicateNibble\b",
        r"(?i)^Returns the (multiplicative|product)\b",
        r"(?i)^Walked too far\b",
        r"(?i)^xlp xep\b",
        r"(?i)^yyunput\b",
        r"(?i)^yylval\b",
        r"(?i)^\?1:0$",
        r"(?i)^\(\(DBus",
        r"(?i)^\(unsigned char\)",
        r"(?i)^16 \(\(d\)\b",
        r"(?i)^l \(unsigned\b",
        r"(?i)^\(\(unsigned\b",
        // ICS false positive copyrights
        r"(?i)^\(c\) \(unsigned int\)",
        r"(?i)^\(c\) A &&",
        r"(?i)^\(c\) a &&",
        r"(?i)^COPYRIGHT undef\b",
        r"(?i)^\(c\) \(\(DBusCondVar",
        r"(?i)^\(c\) s-$",
        r"(?i)^\(c\) A1$",
        r"(?i)^\(c\) this-\s*set\w+\b",
        r"(?i)^\(c\) \(unsigned\)$",
        r"(?i)^COPYRIGHT CREDITS\b",
        r"(?i)^COPYRIGHT HOLDERS,?\s*AND/OR\b",
        r"(?i)^COPYRIGHT exploring\b",
        r"(?i)^Copyright,?\s*lack of warranty\b",
        r"(?i)^COPYRIGHT const char\b",
        r"(?i)^copyright const char\b",
        r"(?i)^copyright mea-\s*setOffset\b",
        r"(?i)^copyright meta-\s*registerClass\b",
        r"(?i)^\(c\) \(unsigned char\)\(",
        r"(?i)^\(c\) \d+L$",
        r"(?i)^\(c\) cvPoint3D32f$",
        r"(?i)^\(c\) temp3$",
        r"(?i)^http://\S+\s+Copyright\b",
        r"(?i)^Foundation Copyright\b",
        r"(?i)^http://sizzlejs\b",
        r"(?i)^\(c\) \(unsigned char\)$",
        r"(?i)@remark Read",
        r"(?i)\bWritten by\b",
        r"(?i)\bcontributors Thomas Broyer\b",
        r"(?i), and are$",
        // Garbled/binary data patterns (junk-copyright-* tests)
        r"^\(c\) Io\\0",
        r"^\(c\) AaeaMOOAA\d",
        r"^\(c\) EEIaeIaAAOAE",
        r"^\(c\) AaACEEeUB",
        r"^\(c\) AIuaey",
        r"^\(c\) ATo\b",
        r"^\(c\) U Q\d",
        r"^\(c\) Vo\b.*\bAoa\b",
        r"^\(c\) Y Rd$",
        r"^\(c\) YY ThQ",
        r"^\(c\) ZIgd\d",
        r"^\(c\) OCOthDTh",
        r"^\(c\) IoUOi",
        r"^\(c\) OthO$",
        r"^\(c\) ErXA\d",
        r"(?i)^\(c\) Dean$",
        r"(?i)^Copyright \(c\) The team$",
        r"^\(c\) 1 \?\d",
        r"^\(c\) 34 b$",
        r"^\(c\) A - 10 a - 10$",
        r"(?i)^\(c\) AS z$",
        // French legal text fragments
        r"(?i)^\(c\) dig[ÃA]",
        r"(?i)^\(c\) que le pr[ÃA]",
        r"(?i)^\(c\) s en anglais",
        r"(?i)^\(c\) sent contrat\b",
        // Garbled text with (c) in middle
        r"(?i)^Xz\b.*\(c\)\s*Ijr",
        // Binary data from image files
        r"^\(c\) [^\x20-\x7e]{2}",
        r"(?i)^COPYRIGHT AS$",
        r"^\(c\) E QuGU",
        r"^\(c\) YY$",
        // (c) followed by non-ASCII byte (binary garbage from image/font files)
        r"^\(c\) [a-zA-Z]{1,3}[\x00-\x1f\x80-\xff]",
        r"[\x00-\x08]",
    ];
    patterns.iter().filter_map(|p| Regex::new(p).ok()).collect()
});

/// Regex patterns for junk holder detections (license boilerplate fragments).
static HOLDERS_JUNK_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let patterns = [
        r"(?i)\bliable for\b",
        r"(?i)\bappear in all copies\b",
        r"(?i)\bdisclaimer of warranty\b",
        r"(?i)\bdisclaimer for the program\b",
        r"(?i)\bit may be distributed\b",
        r"(?i)\bwho places the program\b",
        r"(?i)\bkeep intact all the\b",
        r"(?i)\bshall not be used in advertising\b",
        r"(?i)\bpromote the sale\b",
        r"(?i)\bpromote products derived\b",
        r"(?i)\bother dealings in\b",
        r"(?i)\bhas been advised of the possibility\b",
        r"(?i)\bfailure of essential purpose\b",
        r"(?i)\bthe licenses? granted in\b",
        r"(?i)\bcovering the original code\b",
        r"(?i)\bwithout notice from apple\b",
        r"(?i)\bcompletely and accurately document\b",
        r"(?i)\bother proprietary\b",
        r"(?i)\bpatent rights?\b",
        r"(?i)\bincluding.{0,10}but not limited\b",
        r"(?i)\bincluding your\b",
        r"(?i)\bincluding the\b",
        r"(?i)\bcopyrighted material\b",
        r"(?i)\bselected patent\b",
        r"(?i)\bin the work\b",
        r"(?i)\bin the document\b",
        r"(?i)\bthe original work\b",
        r"(?i)\bpermit and encourage\b",
        r"(?i)\bpermitted copying\b",
        r"(?i)\bto do the following\b",
        r"(?i)\bas a result of\b",
        r"(?i)\breinstated permanently\b",
        r"(?i)\breinstated\b",
        r"(?i)\bexplicitly and finally terminates\b",
        r"(?i)\bfails to notify\b",
        r"(?i)\bnotifies\b",
        r"(?i)\bthe above\b",
        r"(?i)\bthe software,?$",
        r"(?i)\bsuspend your rights\b",
        r"(?i)\bderivative works\b",
        r"(?i)\bpublicly display\b",
        r"(?i)\bpublicly perform\b",
        r"(?i)\bof competent jurisdiction\b",
        r"(?i)\bexceptions and limitations\b",
        r"(?i)\bfair use\b",
        r"(?i)\bfair dealing\b",
        r"(?i)\btreaty adopted\b",
        r"(?i)\breflecting the\b",
        r"(?i)\bappears? in\b",
        r"(?i)\bsaying\b.*\bdistributed\b",
        r"(?i)\bif the item a binary\b",
        r"(?i)\bone digital image or graphic\b",
        r"(?i)\bperceptible, measurable\b",
        r"(?i)\bthe entire\b",
        r"(?i)\bsemblance of artistic control\b",
        r"(?i)\bcommercially reasonable efforts\b",
        r"(?i)\bto endorse or promote\b",
        r"(?i)\bimmediately at the beginning\b",
        r"(?i)\bunmodified\b",
        r"(?i)\beasier identification\b",
        r"(?i)\b(l?gpl|lgpl) group\b",
        r"(?i)^symbol in\b",
        r"(?i)^trademark$",
        r"(?i)^printf\b",
        r"(?i)^the top level of\b",
        r"(?i)^the following\b",
        r"(?i)^the resulting\b",
        r"(?i)^whoever named in\b",
        r"(?i)^as specified below\b",
        r"(?i)^not used to limit\b",
        r"(?i)^the coordinator$",
        r"(?i)^provided\b",
        r"(?i)^provides the work\b",
        r"(?i)\bthis\.[a-zA-Z]",
        r"(?i):function\b",
        r"(?i)\bm\. y\. name\b",
        r"(?i)^version of nameif\b",
        r"(?i)\bunless explicitly identified\b",
        r"(?i)^version 3 of the$",
        // Holder false positives from license boilerplate
        r"(?i)\b(if any) with\b",
        r"(?i)^(d),\b",
        r"(?i)\bas a market\b",
        r"(?i)\bprocedures\b",
        r"(?i)\bcollectively\b",
        r"(?i)\bgiving your\b",
        r"(?i)\bspecified addresses\b",
        r"(?i)^the base\b",
        r"(?i)^the library\b",
        r"(?i)\bthe library,\b",
        r"(?i)\bthe library among\b",
        r"(?i)\breferences to\b",
        r"(?i)\bstating\b.*\bdistributed\b",
        r"(?i)^terminate\b",
        r"(?i)\beffective immediately\b",
        r"(?i)^keep intact\b",
        r"(?i)^material outside\b",
        r"(?i)\bsaying\b",
        // Trailing legal text patterns
        r"(?i)\bdistributed under\b",
        r"(?i)\blicensed under\b",
        r"(?i)\bthe terms\b.*\blicense\b",
        r"(?i)\bthe standard version\b",
        // Code-like patterns in holders
        r"(?i)\bif\s*\(",
        r"(?i)\bfunction\s*\(",
        r"(?i)\breturn\b.*\bfunction\b",
        r"(?i)\bvar\s+\w",
        r"(?i)\bthis\.\w+\(",
        // Trailing text patterns in holders
        r"(?i)\bCredited\b",
        r"(?i)\bConverted to\b",
        r"(?i)\breworked by\b",
        r"(?i)\bVarious bits\b",
        r"(?i)\bCopying and distribution\b",
        r"(?i)\bGPLd\b",
        r"(?i)\bLicense-Alias\b",
        r"(?i)\bcontributors Thomas\b",
        r"(?i)\bWritten by\b",
        r"(?i)\bModified by the\b",
        r"(?i)\btitle Legal\b",
        r"(?i)\bContact <",
        r"(?i)\b- Placed\b",
        r"(?i)\bUnder the terms\b",
        r"(?i)\binfo have been\b",
        r"(?i)\bAuthors Havoc\b",
        r"(?i)\bicon support\b",
        r"(?i)\bmaintainer Paolo\b",
        r"(?i)\bfull list\b",
        r"(?i)\bSTATEMENTS AND\b",
        r"(?i)\bAS IS$",
        r"(?i)\bAS IS CONDITION\b",
        r"(?i)\bNOTICES OR THIS\b",
        r"(?i)\bDOCUMENTATION ISC\b",
        r"(?i)\bpixmaps svg\b",
        r"(?i)\bFull text of\b",
        r"(?i)\btransferred to Nokia\b",
        r"(?i)\bAS PER APPLICABLE\b",
        r"(?i)\bSection 105\b",
        r"(?i)\bGNU AGPL\b",
        r"(?i)\bTenable licenses\b",
        r"(?i)\bagreement with the\b",
        r"(?i)\bgives Customer\b",
        r"(?i)\bshall mean\b",
        r"(?i)\bEnterprise Edition\b",
        r"(?i)\bContributing Authors\b",
        r"(?i)\bAll Downstream\b",
        r"(?i)\bSource Code to\b",
        r"(?i)\bPROTECTION AND IS\b",
        r"(?i)\bnot removed\b",
        r"(?i)\bthe GPSD project\b",
        r"(?i)\bversion 3\.1 of\b",
        r"(?i)\bGPL version\b",
        r"(?i)\bCopyright/g\b",
        r"(?i)\bdata/c\.m4\b",
        r"(?i)\binside so it\b",
        r"(?i)\bmatch standard format\b",
        r"(?i)\bin each output\b",
        r"(?i)\bstr::npos\b",
        r"(?i)\btimes in xrange\b",
        r"(?i)\bin zlib\.h\b",
        r"(?i)\ball paragraphs\b",
        r"(?i)\buse, copy, modify\b",
        r"(?i)\bdistribute it with\b",
        r"(?i)\bother intellectual property\b",
        r"(?i)\btreaties\. Title\b",
        r"(?i)\bexempting the\b",
        r"(?i)\bwith exception of\b",
        r"(?i)\bas noted in the\b",
        r"(?i)\bThe Product is\b",
        r"(?i)\bThe arguments as\b",
        r"(?i)\bpertaining to distribution\b",
        r"(?i)\bVERBATIM\b",
        r"(?i)\bintact$",
        r"(?i)\binformation\.\b",
        r"(?i)\bdoing$",
        r"(?i)^holders,? but\b",
        r"(?i)^its author\b",
        r"(?i)^in its\b",
        r"(?i)^in the\b",
        r"(?i)^offer\b",
        r"(?i)^copy the\b",
        r"(?i)^owned by\b",
        r"(?i)^the team$",
        r"(?i)^the project$",
        r"(?i)^the republic of\b",
        r"(?i)^the google\b",
        r"(?i)^the jetty\b",
        r"(?i)^the acknowledgment\b",
        r"(?i)^the combination of\b",
        r"(?i)^the lavantech\b",
        r"(?i)^all source code\b",
        r"(?i)^all translated\b",
        r"(?i)^all the rich\b",
        r"(?i)^author,? or contributor\b",
        r"(?i)^authors,? and contributors\b",
        r"(?i)^its authors\b",
        r"(?i)^its cell\b",
        r"(?i)^automatically without\b",
        r"(?i)^more information\b",
        r"(?i)^infringement can\b",
        r"(?i)^header of\b",
        r"(?i)^const (group|projects)\b",
        r"(?i)^there clear\b",
        r"(?i)^things like\b",
        r"(?i)^custom nessus\b",
        r"(?i)^whenever reasonably\b",
        r"(?i)^united states government as represented\b",
        r"(?i)^gnu general\b",
        r"(?i)^general public\b",
        r"(?i)^man page\b",
        r"(?i)^merged arm\b",
        r"(?i)^tcl/tk policy\b",
        r"(?i)^in license\b",
        r"(?i)^law,? \b",
        r"(?i)^license,? to the\b",
        r"(?i)^s status\b",
        r"(?i)^as i developed\b",
        r"(?i)^improved syntax\b",
        r"(?i)^inclusion in\b",
        r"(?i)^disclaim all\b",
        r"(?i)^directly copied\b",
        r"(?i)^as found in\b",
        r"(?i)^years updated\b",
        r"(?i)\bcontrol over the development\b",
        r"(?i)\bartistic control\b",
        r"(?i)\bcompilation not used to limit\b",
        r"(?i)\blegal rights of the compilation\b",
        r"(?i)\bindividual works permit\b",
        r"(?i)\bDocument included in\b",
        r"(?i)\blocated in .* and .* located in\b",
        r"(?i)\binternational treaty\b",
        r"(?i)\bapplicable$",
        r"(?i)\bcontrat et tous\b",
        r"(?i)\ben anglais\b",
        r"(?i)\bdocuments connexes\b",
        r"(?i)^seek a different\b",
        r"(?i)^sign so\b",
        r"(?i)^like sta\b",
        r"(?i)^page i/o\b",
        r"(?i)^\(mon tue\b",
        r"(?i)^gt\. zero\b",
        r"(?i)^with recursive\b",
        r"(?i)^ecfieldelement\b",
        r"(?i)^setresultsname\b",
        r"(?i)^semanticdirection\b",
        r"(?i)^content ssense\b",
        r"(?i)^attr value\b",
        r"(?i)^match\(ident\)\b",
        r"(?i)^assert\.equal\b",
        r"(?i)^h\.matches\b",
        r"(?i)^bd\(b\.\b",
        r"(?i)^b\(an\)\d",
        r"(?i)^b\(ase\b",
        r"(?i)^b\(onstant\b",
        r"(?i)^g\(al\)\b",
        r"(?i)^y fj\b",
        r"(?i)^y fp\b",
        r"(?i)^u r\(\d",
        r"(?i)^u q\d",
        r"(?i)^y rd\b",
        r"(?i)^y aey\b",
        r"(?i)^as z$",
        r"(?i)^i\. uao\b",
        r"(?i)^e qugu\b",
        r"(?i)^bj d\b",
        r"(?i)^cj d\b",
        r"(?i)^dj d\b",
        r"(?i)^jj d\b",
        r"(?i)^objc,? bp\b",
        r"(?i)^10 a - 10$",
        r"(?i)^b a, b$",
        r"(?i)^unknown [a-z]{1,3}$",
        r"(?i)^unknown [a-z]\d\b",
        r"(?i)^[a-z]{1,2} [a-z]{1,2}$",
        r"(?i)^ato\b.*\bae\b",
        r"(?i)^xz\b.*\bijr\b",
        r"(?i)^zigd\d\b",
        r"(?i)^yy thq\b",
        r"(?i)^ss'ss",
        r"(?i)^pspsy\b",
        r"(?i)^oo2\b",
        r"(?i)^c/ps\b",
        r"(?i)^cn:class\b",
        r"(?i)^c2001\b",
        r"(?i)^ocoo\b",
        r"(?i)^a!\b",
        r"(?i)^aae\b",
        r"(?i)^a\(r\)\b",
        r"(?i)^deg,?\b.*deg\b",
        r"(?i)^cii1/4\b",
        r"(?i)^vo u\d",
        r"(?i)^ul\b",
        r"(?i)^xl\b",
        r"(?i)^wl\b",
        r"(?i)^crarr\b",
        r"(?i)^x\$\?\b",
        r"(?i)^e\$\?\b",
        r"(?i)^length\?null\b",
        r"(?i)^c\.warn\b",
        r"(?i)^b\.status\b",
        r"(?i)^as\(c,\b",
        r"(?i)^cc\.fr$",
        r"(?i)^q ltd$",
        r"(?i)^zbar\b",
        r"(?i)^ssssy$",
        r"(?i)^ssss$",
        r"(?i)^as5$",
        r"(?i)^r'b$",
        r"(?i)^\?12$",
        r"(?i)^tj et\b",
        r"(?i)^adobe.*\bairtm\b",
        r"(?i)^adobe.*\bair\u{2122}\b",
        r"(?i)^xerox corporation$",
        r"(?i)^david j\. bradshaw$",
        r"(?i)^gias kay lee$",
        r"(?i)^tim ruffles$",
        r"[\x00-\x1f]",
        r"°°°",
        r"ÿÿÿ",
        r"\u{9a}f",
        r"\u{96}b",
        r"\u{9d}v",
        r"^[A-Z][a-z]$",
        r"^[A-Z][b-z]$",
        r"^[a-z][A-Z]$",
        r"^holder\.\b",
        r"^holder,\b",
        r"^holders,\b",
        r"^holder as\b",
        r"(?i)^applied to\b",
        r"(?i)^designation\b",
        r"(?i)^registered$",
        r"(?i)^component$",
        r"(?i)^count$",
        r"(?i)^group$",
        r"(?i)^isupper$",
        r"(?i)^folded$",
        r"(?i)^dean$",
        r"(?i)^targetpath$",
        r"(?i)^libre-software$",
        r"(?i)^\(2\)\.\s*if\b",
        r"(?i)^\(as found in\b",
        r"(?i)^\(directly copied\b",
        r"(?i)^\(if any\)\b",
        r"(?i)^m\(h",
        r"(?i)^b\(onsisting\b",
        r"(?i)^inria-enpc\b",
        r"(?i)^uossoo\b",
        r"(?i)^ocothd\b",
        r"(?i)^otho\b",
        r"(?i)^iouoi\b",
        r"(?i)^aiuaey\b",
        r"(?i)^aoth\b",
        r"(?i)^ato\b",
        r"(?i)^aaeamooa\b",
        r"(?i)^eeiaeiaaoa\b",
        r"(?i)^exauauuao\b",
        r"(?i)^erxa\d",
        r"(?i)^ijax\b",
        r"(?i)^u1e\b",
        r"(?i)^degu\b",
        r"(?i)^xmlns\b",
        r"(?i)^http://www\.quirksmode\b",
        r"(?i)^\u{201c}adobe\b",
        r"(?i)\bthe resulting\b",
        r"(?i)\ball source code included in\b",
        r"(?i)\bsource code distributed need not\b",
        r"(?i)\bdo not make\b",
        r"(?i)\bgive all recipients\b",
        r"(?i)\brecipients of the\b",
        r"(?i)\bmay be used\b",
        r"(?i)\bthe accompanying\b",
        r"(?i)\bas represented by no\b",
        r"(?i)^compute hessian\b",
        r"(?i)^nat\d+\.is",
        r"(?i)^a\.compatibility\b",
        r"(?i)^opencensus authors \d",
        r"(?i)^retained at the\b",
        r"(?i)^timer code\b",
        r"(?i)^ds status works\b",
        r"(?i)^an sr-iov\b",
        r"(?i)^applies to the regex\b",
        r"(?i)^apple's sf pro\b",
        r"(?i)^xmlns:\?",
        r"(?i)^swfobject\b",
        r"(?i)^program\b.*\btalke studio\b",
        r"(?i)^debian\b.*\bjames troup\b",
        r"(?i)^\$id\$\b",
        r"(?i)^the uc berkeley\b",
        r"(?i)^ococoa\b",
        r"(?i)^grant\. i\b",
        r"(?i)^the gnome libraries\b",
        r"(?i)^as is group\b",
        r"(?i)^match\(ident\)\s*ast\b",
        r"(?i)^holder,? author,? or contributor\b",
        r"(?i)^holders,? authors,? and contributors\b",
        r"(?i)\bportions of\b",
        r"(?i)\bsome parts of\b",
        r"(?i)\bthe source$",
        r"(?i)\bthe source code\b",
        r"(?i)\b\. the source\b",
        r"(?i)^p b i n do$",
        r"(?i)^tue \w+ \d+ \w+ \w+ -",
        r"(?i)^info for$",
        r"(?i)^material,? only\b",
        r"(?i)^(d),? \d",
        r"(?i)^c\. schmidt$",
        r"(?i)^gdb free software\b",
        r"(?i)^va$",
        r"(?i)^wing$",
        r"(?i)^hillion$",
        r"(?i)^(TOUPPER|isascii|iscntrl|isprint|yyunput|ambiguous|TRUE FALSE)$",
        r"(?i)^(width|len|do|date|year|note|update|notive|all the)$",
        r"(?i)^undef\s+\w+$",
        r"(?i)^i\.e\.,\b",
        r"(?i)^endif\b",
        r"(?i)^definedummyword\b",
        r"(?i)^register int\b",
        r"(?i)^l \(unsigned\b",
        r"(?i)^\(\(unsigned\b",
        r"(?i)^notices all\b",
        r"(?i)^may not be removed\b",
        r"(?i)^duplicated in\b",
        r"(?i)^copyright for a\b",
        r"(?i)^copyright info\b",
        r"(?i)^COPYRIGHT HOLDERS AS\b",
        r"(?i)^Mouse Wheel Support\b",
        r"(?i)^Joseph Gil avalable\b",
        r"(?i)^Original code for Bayer\b",
        r"(?i)@remark Read the",
        r"(?i)\bEND END$",
        r"(?i)^inc\.,\s*Id Software\b",
        r"(?i)^Id Software.*Id Software\b",
        r"(?i)\bavalab?le at\b",
        r"(?i)^\(k \d+ k \d+\b",
        r"(?i)^\(unsigned char\)\b",
        r"(?i)^\(int\) TOUPPER\b",
        r"(?i)^(isascii|isdigit|isalpha|isalnum|isupper|islower|isspace|isgraph|ispunct|isxdigit)\b",
        r"(?i)^ungetc\b",
        r"(?i)^yylval\b",
        r"(?i)^symrec\b",
        r"(?i)^arg\s*\+\+",
        r"(?i)^gunichar\b",
        r"(?i)^TRUE FALSE$",
        r"(?i)^undef\b",
        r"(?i)^0 1$",
        r"(?i)^\d{1,2}$",
        r"(?i)^ok-for-header$",
        r"(?i)^date\b.*\bDon't assume\b",
        r"(?i)^notive in the\b",
        r"(?i)\bdon't assume ascii\b",
        r"(?i)^all the$",
        r"(?i)\bftp://\b",
        r"(?i)^CC Computer Consultants\b.*\bContact\b",
        r"(?i)^16 \(\(d\)\b",
        r"(?i)^\(c\) s-$",
        r"(?i)^z \?$",
        r"(?i)^Z \?$",
        r"(?i)^this-\s*setStencil\b",
        r"(?i)^temp\d+$",
        r"(?i)^table\.set\b",
        r"(?i)^strict!?\s*-?\d",
        r"(?i)^slen$",
        r"(?i)^save to iv\b",
        r"(?i)^r,?\s*div\b",
        r"(?i)^r sround\b",
        r"(?i)^r \(s\)$",
        r"(?i)^put chain\b",
        r"(?i)^problem,?\s*work-around\b",
        r"(?i)^prec prec\b",
        r"(?i)^pr this\b",
        r"(?i)^Paul Rusty Russell\b.*\bPlaced\b",
        r"(?i)^Paul Mackerras\b.*\bpipe read\b",
        r"(?i)^packet$",
        r"(?i)^p can be called\b",
        // ICS false positives: code fragments, boilerplate, gibberish
        r"(?i)^the parens part of\b",
        r"(?i)^i\.e\.\s*,?\s*call the\b",
        r"(?i)^8 \(\(b\)\b",
        r"(?i)^\d+ \(trail\b",
        r"(?i)^\d+ illegal\b",
        r"(?i)^strict!\s",
        r"(?i)^\(s\) \(i\)$",
        r"(?i)^0x[0-9a-fA-F]+",
        r"(?i)^\d+ \+0x",
        r"(?i)^\d+ &0x",
        r"(?i)^it a lead surrogate\b",
        r"(?i)^uint\d+$",
        r"(?i)^Construct a set of\b",
        r"(?i)^clause removed\b",
        r"(?i)^0x\d",
        r"(?i)^below\.?\s*(Please)?\b",
        r"(?i)^below$",
        r"(?i)^above$",
        r"(?i)^\(qbuf\b",
        r"(?i)^applies to code\b",
        r"(?i)^0x7f\b",
        r"(?i)^\(with the right granted\b",
        r"(?i)^fi$",
        r"(?i)^as follows$",
        r"(?i)^dst$",
        r"(?i)^dst-\s",
        r"(?i)^< 0 e->",
        r"(?i)^\(\d+ \(pattern\b",
        r"(?i)^ACM and IEEE\b",
        r"(?i)^make it clear$",
        r"(?i)^ifdef$",
        r"(?i)^exploring the\b.*\bcultural\b",
        r"(?i)^do prec\b",
        r"(?i)^EOF &&\b",
        r"(?i)^4\+\(r\)$",
        r"(?i)^c&0x",
        r"(?i)^cp\)$",
        r"(?i)^ctype$",
        r"(?i)^macroptr\b",
        r"(?i)^\(shf\)\b",
        r"(?i)^MAGIC$",
        r"(?i)^out\.ro$",
        r"(?i)^attribution$",
        r"(?i)^\d+ ,\s*l\b.*\b(unsigned|endif)\b",
        r"(?i)^asm bswapl\b",
        r"(?i)^d'\(l\)$",
        r"(?i)^l endif$",
        r"(?i)^03o$",
        r"(?i)^apply$",
        r"(?i)^\(cp\)$",
        r"(?i)^buflen\b.*\bbuf$",
        r"(?i)^buffer\s+[a-z]$",
        r"(?i)^etc\b.*\bstrings\b",
        r"(?i)^1,\s*cls\.\b",
        r"(?i)^this-\s*set\w+\b",
        r"(?i)^\(scale\s+\d\)\s+\d",
        r"(?i)^fWidth$",
        r"(?i)^dst \d$",
        r"(?i)^i \(s\)\b.*\b(while|endif)\b",
        r"(?i)^i ,?\s*div\b",
        r"(?i)^i sround\b",
        r"(?i)^i,?\s*s while\b",
        r"(?i)^r,?\s*div\b",
        r"(?i)^r,?\s*s$",
        r"(?i)^\(a\) \(b\)$",
        r"(?i)^FILE\.*\s+\w+\.\w+\s+AUTHOR\b",
        r"(?i)^Accumulate$",
        r"(?i)^ctable\b",
        r"(?i)^CREDITS PORTING\b",
        r"(?i)^24 endif$",
        r"(?i)^\^ 0x\d",
        r"(?i)^putchar\b.*\bputchar\b",
        r"(?i)^do while$",
        r"(?i)^\^ \(b\)$",
        r"(?i)^help$",
        r"(?i)^in gzlog\.h\b",
        r"(?i)^decoded by\b",
        r"(?i)^IBM Corporation\.$",
        r"(?i)^Lotus Development Corporation\.$",
        r"(?i)^disclaimer$",
        r"(?i)^disclaims all\b",
        r"(?i)^Foundation IBM\b",
        r"(?i)^http://sizzlejs\b",
        r"(?i)^Kasım$",
        r"(?i)^Akim Demaille$",
        r"(?i)^Joel E\. Denny$",
        r"(?i)^num \d$",
        r"(?i)^letters\b.*\bc - A\b",
        r"(?i)^classify$",
        r"(?i)^\(r\) l l$",
        r"(?i)^ok letters\b.*\bcond\b",
        r"(?i)^flags ptbl-\b",
        r"(?i)^holders,?\s*disclaims\b",
        r"(?i)^\d+L$",
        r"(?i)^\d+L,\s*l\b",
        r"(?i)^Chain has\b",
        r"(?i)^DEBUGP\b",
        r"(?i)^Only user$",
        r"(?i)^cindex chains$",
        r"(?i)^foot-\s*target\b",
        r"(?i)^OProfile authors\b.*@remark",
        r"(?i)^and are$",
        r"(?i)^mea-\s*setOffset\b",
        r"(?i)^meta-\s*registerClass\b",
        r"(?i)^EOF &&\s",
        r"(?i)^Bit8u$",
        r"(?i)^cvPoint3D32f$",
        r"(?i)^G2P ADJ\b",
        r"(?i)^info to be inserted\b",
        r"(?i)^0 isupper$",
        r"(?i)^0-9,- \d",
        r"(?i)^97 static$",
        r"(?i)^8 \(\(b\)\s*\d",
        r"(?i)^\d+ \(\(d\)\s*\d",
        r"(?i)^\d+ \(\(a\)\s*\d",
        r"(?i)\b@version \$Id\b",
        r"(?i)^ds Status works\b",
        r"(?i)^holder\.\s*AS\b",
        r"(?i)^oCOOA\b",
        r"(?i)^as\(c,\s*field\b",
        r"(?i)^a!\s*\+-",
        r"(?i)^\(xmlns:\?\s*\^",
        r"(?i)^Tue\s+\w+\s+\d+\s+\w+\s+\w+\s+<",
        r"(?i)\b\d{2}-[A-Z]{3}-\d{2}\s+Bugfixes\b",
        r"(?i)\bpartial mlock\b",
        r"(?i)\bskb\.\s*The buffer\b",
        r"(?i)^IBM Corp\.\s*Auxtrace\b",
        r"(?i)^digÃ",
        r"(?i)^que le prÃ",
        // Garbled binary data holders (from junk-copyright tests)
        r"^AaACEEeUB",
        r"^AaeaMOOAA\d",
        r"^EEIaeIaAAOAE",
        r"^OCOthDTh",
        r"^YY ThQ",
        r"^YY$",
        r"(?i)^NIST\.\d+\.\d+\.",
        r"(?i)AEEEUAU",
        r"(?i)\$\?I\$\?i\$\?I",
        r"^x!C/!O$",
    ];
    patterns.iter().filter_map(|p| Regex::new(p).ok()).collect()
});

// ─── Junk detection ──────────────────────────────────────────────────────────

/// Return true if `s` matches any known junk copyright pattern.
pub fn is_junk_copyright(s: &str) -> bool {
    COPYRIGHTS_JUNK_PATTERNS.iter().any(|re| re.is_match(s))
}

/// Return true if `s` matches any known junk holder pattern.
fn is_junk_holder(s: &str) -> bool {
    HOLDERS_JUNK_PATTERNS.iter().any(|re| re.is_match(s))
}

// ─── Core refinement functions ───────────────────────────────────────────────

/// Refine a detected copyright string. Returns `None` if the result is empty.
pub fn refine_copyright(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let mut c = normalize_whitespace(s);
    c = strip_some_punct(&c);
    c = strip_solo_quotes(&c);
    // strip trailing slashes, tildes, spaces
    c = c.trim_matches(&['/', ' ', '~'][..]).to_string();
    c = strip_all_unbalanced_parens(&c);
    c = remove_some_extra_words_and_punct(&c);
    c = normalize_whitespace(&c);
    c = remove_dupe_copyright_words(&c);
    c = strip_prefixes(&c, &HashSet::from(["by", "c"]));
    c = c.trim().to_string();
    c = c.trim_matches('+').to_string();
    c = strip_balanced_edge_parens(&c).to_string();
    c = strip_suffixes(&c, &COPYRIGHTS_SUFFIXES);
    c = strip_trailing_period(&c);
    c = c.trim_matches('\'').to_string();
    c = strip_trailing_url_slash(&c);
    c = truncate_long_words(&c);
    let result = c.trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Refine a detected holder name. Returns `None` if junk or empty.
pub fn refine_holder(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }

    // Choose prefix set based on whether "reserved" appears.
    let prefixes = if s.to_lowercase().contains("reserved") {
        &*HOLDERS_PREFIXES_WITH_ALL
    } else {
        &*HOLDERS_PREFIXES
    };

    let mut h = s.replace("build.year", " ");

    // Strip leading date-like prefix (digits, dashes, slashes).
    if h.contains(' ')
        && let Some((prefix, suffix)) = h.split_once(' ')
        && prefix
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == '/')
    {
        h = suffix.to_string();
    }

    h = remove_some_extra_words_and_punct(&h);
    h = h.trim_matches(&['/', ' ', '~'][..]).to_string();
    h = refine_names(&h, prefixes);
    h = strip_suffixes(&h, &HOLDERS_SUFFIXES);
    h = h.trim_matches(&['/', ' ', '~'][..]).to_string();
    h = strip_solo_quotes(&h);
    h = h.replace("( ", " ").replace(" )", " ");
    h = h.trim_matches(&['+', '-', ' '][..]).to_string();
    h = strip_trailing_period(&h);
    h = h.trim_matches(&['+', '-', ' '][..]).to_string();
    h = remove_dupe_holder(&h);
    h = normalize_whitespace(&h);
    h = strip_trailing_url(&h);
    h = h.trim_matches(&[',', ' '][..]).to_string();
    h = normalize_whitespace(&h);
    h = truncate_long_words(&h);
    h = h.trim().to_string();

    if !h.is_empty() && !HOLDERS_JUNK.contains(h.to_lowercase().as_str()) && !is_junk_holder(&h) {
        Some(h)
    } else {
        None
    }
}

/// Refine a detected author name. Returns `None` if junk or empty.
pub fn refine_author(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let mut a = remove_some_extra_words_and_punct(s);
    a = refine_names(&a, &AUTHORS_PREFIXES);
    a = a.trim().to_string();
    a = strip_trailing_period(&a);
    a = a.trim().to_string();
    a = strip_balanced_edge_parens(&a).to_string();
    a = a.trim().to_string();
    a = strip_solo_quotes(&a);
    a = refine_names(&a, &AUTHORS_PREFIXES);
    a = a.trim().to_string();
    a = a.trim_matches(&['+', '-'][..]).to_string();

    if !a.is_empty()
        && !AUTHORS_JUNK.contains(a.to_lowercase().as_str())
        && !a.starts_with(AUTHORS_JUNK_PREFIX)
        && !is_junk_author(&a)
    {
        Some(a)
    } else {
        None
    }
}

/// Refine a name string (shared logic for holders and authors).
fn refine_names(s: &str, prefixes: &HashSet<&str>) -> String {
    let mut r = strip_some_punct(s);
    r = strip_leading_numbers(&r);
    r = strip_all_unbalanced_parens(&r);
    r = strip_some_punct(&r);
    r = r.trim().to_string();
    r = strip_balanced_edge_parens(&r).to_string();
    r = r.trim().to_string();
    r = strip_prefixes(&r, prefixes);
    r = strip_some_punct(&r);
    r = r.trim().to_string();
    r
}

// ─── Helper / utility functions ──────────────────────────────────────────────

/// Normalize whitespace: collapse runs of whitespace to single spaces.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Remove duplicate/variant copyright words and normalize them.
pub fn remove_dupe_copyright_words(c: &str) -> String {
    let mut c = c.to_string();
    c = c.replace("SPDX-FileCopyrightText", "Copyright");
    c = c.replace("SPDX-SnippetCopyrightText", "Copyright");
    c = c.replace("Bundle-Copyright", "Copyright");
    c = c.replace("AssemblyCopyright", "Copyright");
    c = c.replace("AppCopyright", "Copyright");
    c = c.replace("Cppyright", "Copyright");
    c = c.replace("cppyright", "Copyright");

    // Various prefix to the word copyright seen in binaries.
    for prefix in &["B", "E", "F", "J", "M", "m", "r", "V"] {
        let from = format!("{prefix}Copyright");
        c = c.replace(&from, "Copyright");
    }
    c = c.replace("JCOPYRIGHT", "Copyright");

    // Duplicate copyright words from markup artifacts.
    c = c.replace("COPYRIGHT Copyright", "Copyright");
    c = c.replace("Copyright Copyright", "Copyright");
    c = c.replace("Copyright copyright", "Copyright");
    c = c.replace("copyright copyright", "Copyright");
    c = c.replace("copyright Copyright", "Copyright");
    c = c.replace("copyright'Copyright", "Copyright");
    c = c.replace("copyright\"Copyright", "Copyright");
    c = c.replace("copyright' Copyright", "Copyright");
    c = c.replace("copyright\" Copyright", "Copyright");
    c = c.replace("Copyright @copyright", "Copyright");
    c = c.replace("copyright @copyright", "Copyright");

    // Broken copyright words.
    c = c.replace("(c) opyrighted", "Copyright (c)");
    c = c.replace("(c) opyrights", "Copyright (c)");
    c = c.replace("(c) opyright", "Copyright (c)");
    c = c.replace("(c) opyleft", "Copyleft (c)");
    c = c.replace("(c) opylefted", "Copyleft (c)");
    c = c.replace("copyright'", "Copyright");
    c = c.replace("and later", " ");
    c = c.replace("build.year", " ");
    c
}

/// Remove miscellaneous junk words and punctuation.
pub fn remove_some_extra_words_and_punct(c: &str) -> String {
    let mut c = c.to_string();
    c = c.replace("<p>", " ");
    c = c.replace("<a href", " ");
    c = c.replace("date-of-software", " ");
    c = c.replace("date-of-document", " ");
    c = c.replace(" $ ", " ");
    c = c.replace(" ? ", " ");
    c = c.replace("</a>", " ");
    c = c.replace("( )", " ");
    c = c.replace("()", " ");
    c = c.replace("__", " ");
    c = c.replace("--", "-");
    c = c.replace(".com'", ".com");
    c = c.replace(".org'", ".org");
    c = c.replace(".net'", ".net");
    c = c.replace("mailto:", "");
    c = c.replace("@see", "");
    if c.ends_with("as represented by")
        && let Some(idx) = c.find("as represented by")
    {
        c = c[..idx].to_string();
    }
    c.trim().to_string()
}

/// Strip leading words that match any of the given prefixes (case-insensitive).
pub fn strip_prefixes(s: &str, prefixes: &HashSet<&str>) -> String {
    let mut words: Vec<&str> = s.split_whitespace().collect();
    while !words.is_empty() && prefixes.contains(words[0].to_lowercase().as_str()) {
        words.remove(0);
    }
    words.join(" ")
}

/// Strip trailing words that match any of the given suffixes (case-insensitive).
pub fn strip_suffixes(s: &str, suffixes: &HashSet<&str>) -> String {
    let mut words: Vec<&str> = s.split_whitespace().collect();
    while !words.is_empty() && suffixes.contains(words.last().unwrap().to_lowercase().as_str()) {
        words.pop();
    }
    words.join(" ")
}

/// Strip trailing period, preserving it for acronyms and company suffixes.
pub fn strip_trailing_period(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() || !s.ends_with('.') {
        return s.to_string();
    }
    // Keep periods for very short strings (acronyms like "P.").
    if s.len() < 3 {
        return s.to_string();
    }

    let is_single_word = s.split_whitespace().count() == 1;
    let bytes = s.as_bytes();

    // U.S.A., e.V., M.I.T. — second-to-last char is uppercase and multi-word.
    if bytes[bytes.len() - 2].is_ascii_uppercase() && !is_single_word {
        return s.to_string();
    }

    // S.A., e.v., b.v. — third-to-last char is a period.
    if bytes.len() >= 3 && bytes[bytes.len() - 3] == b'.' {
        return s.to_string();
    }

    // Company suffixes.
    let lower = s.to_lowercase();
    if lower.ends_with("inc.")
        || lower.ends_with("corp.")
        || lower.ends_with("ltd.")
        || lower.ends_with("llc.")
        || lower.ends_with("co.")
        || lower.ends_with("llp.")
    {
        return s.to_string();
    }

    s.trim_end_matches('.').to_string()
}

/// Strip leading words that are purely digits.
pub fn strip_leading_numbers(s: &str) -> String {
    let mut words: Vec<&str> = s.split_whitespace().collect();
    while !words.is_empty() && words[0].chars().all(|c| c.is_ascii_digit()) {
        words.remove(0);
    }
    words.join(" ")
}

/// Strip some leading and trailing punctuation.
pub fn strip_some_punct(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }
    let s = s.trim_matches(&[',', '\'', '"', '}', '{', '-', '_', ':', ';', '&', '@', '!'][..]);
    let s = s.trim_start_matches(&['.', '>', ')', ']', '\\', '/'][..]);
    let s = s.trim_end_matches(&['<', '(', '[', '\\', '/'][..]);
    s.to_string()
}

/// Replace unbalanced parentheses with spaces for a given pair of delimiters.
pub fn strip_unbalanced_parens(s: &str, open: char, close: char) -> String {
    if !s.contains(open) && !s.contains(close) {
        return s.to_string();
    }

    let mut stack: Vec<usize> = Vec::new();
    let mut unbalanced: Vec<usize> = Vec::new();

    for (i, ch) in s.chars().enumerate() {
        if ch == open {
            stack.push(i);
        } else if ch == close && stack.pop().is_none() {
            unbalanced.push(i);
        }
    }
    // Remaining opens are unbalanced.
    unbalanced.extend(stack);

    if unbalanced.is_empty() {
        return s.to_string();
    }

    let positions: HashSet<usize> = unbalanced.into_iter().collect();
    s.chars()
        .enumerate()
        .map(|(i, c)| if positions.contains(&i) { ' ' } else { c })
        .collect()
}

/// Strip all unbalanced parentheses for (), <>, [], {}.
pub fn strip_all_unbalanced_parens(s: &str) -> String {
    let mut c = strip_unbalanced_parens(s, '(', ')');
    c = strip_unbalanced_parens(&c, '<', '>');
    c = strip_unbalanced_parens(&c, '[', ']');
    c = strip_unbalanced_parens(&c, '{', '}');
    c
}

/// Strip solo quotes in certain contexts.
pub fn strip_solo_quotes(s: &str) -> String {
    s.replace("/'", "/")
        .replace(")'", ")")
        .replace(":'", ":")
        .replace("':", ":")
        .replace("',", ",")
}

/// Strip trailing URL from a string (e.g., "Acme Corp, http://acme.com" → "Acme Corp").
fn strip_trailing_url(s: &str) -> String {
    if let Some(idx) = s.find("http://").or_else(|| s.find("https://")) {
        let before = s[..idx].trim_end_matches(&[',', ' ', ';'][..]);
        if before.is_empty() {
            return s.to_string();
        }
        return before.to_string();
    }
    s.to_string()
}

/// Strip trailing slash from URLs at the end of a string.
/// `"FSF http://fsf.org/"` → `"FSF http://fsf.org"`
fn strip_trailing_url_slash(s: &str) -> String {
    if s.ends_with('/') && (s.contains("http://") || s.contains("https://")) {
        s.trim_end_matches('/').to_string()
    } else {
        s.to_string()
    }
}

/// Remove duplicated holder strings.
fn remove_dupe_holder(h: &str) -> String {
    h.replace(
        "the Initial Developer the Initial Developer",
        "the Initial Developer",
    )
}

/// Drop trailing words longer than 80 characters (garbled/binary data).
fn truncate_long_words(s: &str) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut result: Vec<&str> = Vec::new();
    for w in &words {
        if w.len() > 80 {
            break;
        }
        result.push(w);
    }
    result.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── strip_some_punct ─────────────────────────────────────────────

    #[test]
    fn test_strip_some_punct_basic() {
        assert_eq!(strip_some_punct(",hello,"), "hello");
    }

    #[test]
    fn test_strip_some_punct_leading_dot() {
        assert_eq!(strip_some_punct(".hello"), "hello");
    }

    #[test]
    fn test_strip_some_punct_trailing_paren() {
        assert_eq!(strip_some_punct("hello("), "hello");
    }

    #[test]
    fn test_strip_some_punct_empty() {
        assert_eq!(strip_some_punct(""), "");
    }

    // ── strip_trailing_period ────────────────────────────────────────

    #[test]
    fn test_strip_trailing_period_normal() {
        assert_eq!(strip_trailing_period("Hello World."), "Hello World");
    }

    #[test]
    fn test_strip_trailing_period_inc() {
        assert_eq!(strip_trailing_period("Acme Inc."), "Acme Inc.");
    }

    #[test]
    fn test_strip_trailing_period_ltd() {
        assert_eq!(strip_trailing_period("Foo Ltd."), "Foo Ltd.");
    }

    #[test]
    fn test_strip_trailing_period_acronym() {
        // "e.V." — second-to-last is uppercase, multi-word
        assert_eq!(strip_trailing_period("Foo e.V."), "Foo e.V.");
    }

    #[test]
    fn test_strip_trailing_period_short_acronym() {
        // "b.v." — third-to-last is a period
        assert_eq!(strip_trailing_period("Foo b.v."), "Foo b.v.");
    }

    #[test]
    fn test_strip_trailing_period_no_period() {
        assert_eq!(strip_trailing_period("Hello"), "Hello");
    }

    #[test]
    fn test_strip_trailing_period_short() {
        assert_eq!(strip_trailing_period("P."), "P.");
    }

    #[test]
    fn test_strip_trailing_period_empty() {
        assert_eq!(strip_trailing_period(""), "");
    }

    // ── strip_leading_numbers ────────────────────────────────────────

    #[test]
    fn test_strip_leading_numbers_basic() {
        assert_eq!(strip_leading_numbers("123 456 Hello"), "Hello");
    }

    #[test]
    fn test_strip_leading_numbers_no_numbers() {
        assert_eq!(strip_leading_numbers("Hello World"), "Hello World");
    }

    #[test]
    fn test_strip_leading_numbers_all_numbers() {
        assert_eq!(strip_leading_numbers("123 456"), "");
    }

    // ── strip_prefixes / strip_suffixes ──────────────────────────────

    #[test]
    fn test_strip_prefixes_basic() {
        let prefixes: HashSet<&str> = ["by", "and"].into_iter().collect();
        assert_eq!(strip_prefixes("by and John Doe", &prefixes), "John Doe");
    }

    #[test]
    fn test_strip_suffixes_basic() {
        let suffixes: HashSet<&str> = [".", ",", "and"].into_iter().collect();
        assert_eq!(strip_suffixes("John Doe and", &suffixes), "John Doe");
    }

    // ── strip_unbalanced_parens ──────────────────────────────────────

    #[test]
    fn test_strip_unbalanced_parens_balanced() {
        assert_eq!(
            strip_unbalanced_parens("This is a super(c) string", '(', ')'),
            "This is a super(c) string"
        );
    }

    #[test]
    fn test_strip_unbalanced_parens_unbalanced_close() {
        assert_eq!(
            strip_unbalanced_parens("This )(is a super(c) string)(", '(', ')'),
            "This  (is a super(c) string) "
        );
    }

    #[test]
    fn test_strip_unbalanced_parens_lone_open() {
        assert_eq!(strip_unbalanced_parens("This ( is", '(', ')'), "This   is");
    }

    #[test]
    fn test_strip_unbalanced_parens_lone_close() {
        assert_eq!(strip_unbalanced_parens("This ) is", '(', ')'), "This   is");
    }

    #[test]
    fn test_strip_unbalanced_parens_single_open() {
        assert_eq!(strip_unbalanced_parens("(", '(', ')'), " ");
    }

    #[test]
    fn test_strip_unbalanced_parens_single_close() {
        assert_eq!(strip_unbalanced_parens(")", '(', ')'), " ");
    }

    // ── strip_solo_quotes ────────────────────────────────────────────

    #[test]
    fn test_strip_solo_quotes_url() {
        assert_eq!(
            strip_solo_quotes("https://example.com/'"),
            "https://example.com/"
        );
    }

    #[test]
    fn test_strip_solo_quotes_paren() {
        assert_eq!(strip_solo_quotes("foo)'"), "foo)");
    }

    // ── remove_dupe_copyright_words ──────────────────────────────────

    #[test]
    fn test_remove_dupe_spdx() {
        let result = remove_dupe_copyright_words("SPDX-FileCopyrightText 2024 Acme");
        assert_eq!(result, "Copyright 2024 Acme");
    }

    #[test]
    fn test_remove_dupe_double_copyright() {
        let result = remove_dupe_copyright_words("Copyright Copyright 2024 Acme");
        assert_eq!(result, "Copyright 2024 Acme");
    }

    #[test]
    fn test_remove_dupe_cppyright() {
        let result = remove_dupe_copyright_words("Cppyright 2024 Acme");
        assert_eq!(result, "Copyright 2024 Acme");
    }

    // ── remove_some_extra_words_and_punct ─────────────────────────────

    #[test]
    fn test_remove_extra_words_html() {
        let result = remove_some_extra_words_and_punct("<p>Hello</a>");
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_remove_extra_words_mailto() {
        let result = remove_some_extra_words_and_punct("mailto:foo@bar.com");
        assert_eq!(result, "foo@bar.com");
    }

    #[test]
    fn test_remove_extra_words_as_represented_by() {
        let result = remove_some_extra_words_and_punct("Acme Corp as represented by");
        assert_eq!(result, "Acme Corp");
    }

    // ── is_junk_copyright ────────────────────────────────────────────

    #[test]
    fn test_is_junk_copyright_bare_c() {
        assert!(is_junk_copyright("(c)"));
    }

    #[test]
    fn test_is_junk_copyright_bare_copyright_c() {
        assert!(is_junk_copyright("Copyright (c)"));
    }

    #[test]
    fn test_is_junk_copyright_normal() {
        assert!(!is_junk_copyright("Copyright 2024 Acme Inc."));
    }

    #[test]
    fn test_is_junk_copyright_holder_or_simply() {
        assert!(is_junk_copyright("copyright holder or simply foo"));
    }

    // ── refine_copyright ─────────────────────────────────────────────

    #[test]
    fn test_refine_copyright_basic() {
        let result = refine_copyright("Copyright 2024 Acme Inc.");
        assert_eq!(result, Some("Copyright 2024 Acme Inc.".to_string()));
    }

    #[test]
    fn test_refine_copyright_empty() {
        assert_eq!(refine_copyright(""), None);
    }

    #[test]
    fn test_refine_copyright_strips_junk_prefix() {
        let result = refine_copyright("by Copyright 2024 Acme");
        assert_eq!(result, Some("Copyright 2024 Acme".to_string()));
    }

    // ── refine_holder ────────────────────────────────────────────────

    #[test]
    fn test_refine_holder_basic() {
        let result = refine_holder("Acme Inc.");
        assert_eq!(result, Some("Acme Inc.".to_string()));
    }

    #[test]
    fn test_refine_holder_empty() {
        assert_eq!(refine_holder(""), None);
    }

    #[test]
    fn test_refine_holder_junk() {
        assert_eq!(refine_holder("the"), None);
    }

    #[test]
    fn test_refine_holder_strips_prefix() {
        let result = refine_holder("by Acme Corp");
        assert_eq!(result, Some("Acme Corp".to_string()));
    }

    #[test]
    fn test_refine_holder_strips_trailing_period() {
        let result = refine_holder("IBM Corporation.");
        assert_eq!(result, Some("IBM Corporation".to_string()));
    }

    // ── refine_author ────────────────────────────────────────────────

    #[test]
    fn test_refine_author_basic() {
        let result = refine_author("John Doe");
        assert_eq!(result, Some("John Doe".to_string()));
    }

    #[test]
    fn test_refine_author_empty() {
        assert_eq!(refine_author(""), None);
    }

    #[test]
    fn test_refine_author_junk() {
        assert_eq!(refine_author("james hacker"), None);
    }

    #[test]
    fn test_refine_author_strips_author_prefix() {
        let result = refine_author("author John Doe");
        assert_eq!(result, Some("John Doe".to_string()));
    }

    #[test]
    fn test_refine_author_junk_prefix() {
        assert_eq!(refine_author("httpProxy something"), None);
    }

    // ── strip_all_unbalanced_parens ──────────────────────────────────

    #[test]
    fn test_strip_all_unbalanced_parens_mixed() {
        let result = strip_all_unbalanced_parens("Hello ) World < Foo >");
        // The lone ) and the balanced <> should be handled.
        assert_eq!(result, "Hello   World < Foo >");
    }

    // ── URL slash stripping ──────────────────────────────────────────

    #[test]
    fn test_refine_copyright_url_trailing_slash() {
        let result =
            refine_copyright("Copyright (c) 2007 Free Software Foundation, Inc. http://fsf.org/");
        assert_eq!(
            result,
            Some("Copyright (c) 2007 Free Software Foundation, Inc. http://fsf.org".to_string())
        );
    }
}
