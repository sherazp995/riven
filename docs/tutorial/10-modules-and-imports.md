# Modules and Imports

## Defining Modules

Group related code with `module`:

```riven
module Http
  pub class Request
    url: String
    method: String
    def init(@url: String, @method: String) end
  end

  pub class Response
    status: Int
    body: String
    def init(@status: Int, @body: String) end
  end

  pub def get(url: &str) -> Result[Response, HttpError]
    # ...
  end
end
```

## Nested Modules

```riven
module App
  module Models
    pub class User
      name: String
      def init(@name: String) end
    end
  end

  module Services
    pub def create_user(name: String) -> User
      User.new(name)
    end
  end
end
```

## Importing

### Simple Import

```riven
use Http.Request
use Http.Response
```

### Grouped Import

```riven
use Http.{ Request, Response }
```

### Aliased Import

```riven
use Http.Client as HC
```

### Using Imported Names

```riven
use Http.{ Request, Response }

let req = Request.new("https://example.com", "GET")
```

## Visibility Rules

Items are **private by default**. Mark items as `pub` to make them accessible outside their module:

```riven
module Database
  # Private — only accessible within Database module
  def connect_internal -> Connection
    # ...
  end

  # Public — accessible from outside
  pub def query(sql: &str) -> Result[Rows, DbError]
    let conn = connect_internal()
    # ...
  end
end
```
