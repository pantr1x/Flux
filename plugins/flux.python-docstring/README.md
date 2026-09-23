# Python Docstring

Like **autoDocstring** for VS Code. Under a function, type `"""` and Flux writes the docstring for you:

```python
def area(width: float, height: float = 1) -> float:
    """Short description of area.

    Args:
        width (float): description
        height (float): description

    Returns:
        float: description
    """
```

- Press **Tab** to jump from one description to the next.
- Works with type hints, default values and functions over several lines.
- Or put the cursor on the `def` line and press **Ctrl+Shift+2**.
