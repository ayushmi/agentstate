from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="agentstate",
    version="1.0.0",
    author="Ayush Mittal",
    author_email="ayushsmittal@gmail.com",
    description="Firebase for AI Agents - Persistent state management for AI applications",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/ayushmi/agentstate",
    project_urls={
        "Bug Tracker": "https://github.com/ayushmi/agentstate/issues",
        "Documentation": "https://github.com/ayushmi/agentstate#readme",
        "Source Code": "https://github.com/ayushmi/agentstate",
    },
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "Topic :: Internet :: WWW/HTTP :: HTTP Servers",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
    ],
    keywords="ai agents state management firebase persistent storage real-time",
    packages=find_packages(),
    install_requires=[
        "requests>=2.25.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.0.0",
            "black>=22.0.0",
            "isort>=5.10.0",
            "mypy>=1.0.0",
        ],
        "grpc": [
            "grpcio>=1.50.0",
            "protobuf>=4.0.0",
        ],
    },
    python_requires=">=3.9",
    include_package_data=True,
    zip_safe=False,
)
