use ExtUtils::MakeMaker;
WriteMakefile(
    NAME         => 'Acme::Example',
    VERSION      => '1.23',
    AUTHOR       => 'Jane Smith <[email protected]>',
    ABSTRACT     => 'An example CPAN module',
    LICENSE      => 'perl_5',
    PREREQ_PM    => {
        'Carp'          => 0,
        'File::Spec'    => '3.40',
    },
    BUILD_REQUIRES => {
        'Test::More'    => '0.88',
    },
    MIN_PERL_VERSION => '5.008001',
);
