import os

os.system("echo 'should not execute'")

from setuptools import setup

setup(
    name="malicious",
    version="0.1.0",
)
