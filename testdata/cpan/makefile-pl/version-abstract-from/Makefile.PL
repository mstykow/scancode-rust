use ExtUtils::MakeMaker;

WriteMakefile(
    NAME          => 'Acme::Combined',
    VERSION_FROM  => 'lib/Acme/Combined.pm',
    ABSTRACT_FROM => 'lib/Acme/Combined.pm',
    AUTHOR        => 'Casey Example <casey@example.com>',
    LICENSE       => 'perl_5',
    PREREQ_PM     => {
        'Carp' => 0,
    },
);
