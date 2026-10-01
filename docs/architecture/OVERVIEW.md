# Architecture

Initial architecture:

    CLI
     |
     v
    Application
     |
     +--> Project Core
     |      |
     |      +--> Resources
     |      +--> Tasks
     |      +--> Capsules
     |      +--> Receipts
     |
     +--> Context Compiler
     |
     +--> Manual Bridge
     |
     +--> Verifier
     |
     +--> Store
            |
            +--> SQLite
            +--> Artifact filesystem

The project state is the source of truth.

Provider conversations are execution surfaces, not project storage.
