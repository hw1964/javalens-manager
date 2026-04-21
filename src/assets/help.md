# javalens-manager Help

Welcome to **javalens-manager**, the central hub for managing JavaLens MCP services across your projects.

## Dashboard

The Dashboard is your primary view for managing projects and their runtime states.

*   **Add Project:** Use the form on the left to register a new Java project. You can also import multiple projects at once from a VS Code `.code-workspace` file.
*   **Start / Stop:** Each project has its own dedicated JavaLens MCP server. You can start or stop them individually or use the bulk actions at the top of the list.
*   **Deploy to Agents:** Once your projects are configured, use this toolbar to inject the MCP server configurations into your AI agents (Cursor, Claude, etc.).

## Deploying to Agents

The deploy toolbar provides several actions to manage how your AI agents connect to JavaLens:

*   **Deploy:** Generates the MCP configuration for all running projects and writes it to the selected agents' config files. It also injects MCP-first rule blocks to guide the agent's behavior.
*   **Dry run:** Simulates the deployment process. It validates paths and shows you exactly what would change without actually writing any files.
*   **Regenerate:** Forces a complete rewrite of the manager-owned sections in your agent configs, even if no changes were detected.
*   **Delete:** Removes all manager-managed MCP servers and rule blocks from the target clients.

*Tip: You can select which specific agents to target for a single run using the checkboxes that appear when you click a deploy action.*

## Settings

The Settings view allows you to configure the underlying behavior of the manager:

*   **JavaLens Runtime:** Choose whether to use the automatically managed runtime (downloaded from GitHub) or a local JAR file. You can also check for updates here.
*   **Machine Runtime Controls:** Configure the data root directory (where logs, caches, and tools are stored) and the port range used for assigning ports to new projects.
*   **MCP Config Locations:** Review the automatically detected paths for your AI agents' configuration files. If an agent is installed in a non-standard location, you can provide a manual override path here. You can also set the default deploy targets.
