from setuptools import setup

REQUIREMENTS = ["requests>=2.0", "click"]

setup(
    name="reqpkg",
    version="0.2.0",
    install_requires=REQUIREMENTS,
)
