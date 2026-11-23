# State

The **State** tab shows what Terraform manages. Tuffie reads it with `terraform show -json`, so it works with every backend.

## Resources

- The left list shows every resource, grouped by module. Blue dots are managed resources. Violet dots are data sources.
- Type in **Filter resources** to find one.
- Click a resource to see its attributes on the right.
- **Depends on** lists the resources it needs. Click one to jump to it.

Values that Terraform marks as sensitive show as **sensitive**.

## Graph

Click **Graph** to see the dependency graph.

- Columns show dependency depth. A resource is to the right of everything it depends on.
- Arrows point from a dependency to the resource that uses it.
- Click a resource to highlight its direct links. Click it again to clear.

## Outputs

Click **Outputs** to see the root module outputs.

## State history

Tuffie saves an encrypted copy of the state after every apply. Use the selector next to the tabs:

- **Current state (live):** read now from your backend.
- **Serial N · after …:** the copy saved after that run.

This gives you history even with the `local` backend. Tuffie does not restore old state for you. To roll back, use your backend's own versioning, for example S3 object versions.
