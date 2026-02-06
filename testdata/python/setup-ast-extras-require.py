from setuptools import setup

setup(
    name="extras",
    version="1.0.0",
    extras_require={
        "dev": ["pytest>=7.0", "black>=22.0"],
        "docs": ["sphinx>=5.0"],
    },
    tests_require=["coverage>=6.0"],
)
