#### Cómo utilizar
Esta biblioteca utiliza la API de ZeroBounce, la cual requiere una clave de API. Consulta [esta guía](https://www.zerobounce.net/docs/api-dashboard#API_keys_management) para ver cómo obtener la tuya.

Revisa los [fragmentos de ejemplo](https://github.com/zerobounce/zerobounce-rust-api/tree/main/examples) para ver cómo puedes integrar esta biblioteca en tu propio proyecto.

## Métodos de Búsqueda de Correo Electrónico

### find_email_v2 (Recomendado)

Encuentra una dirección de correo electrónico usando un dominio o nombre de empresa. Utiliza el patrón builder para llamadas API ergonómicas.

**Requisitos:**
- `first_name` es obligatorio
- Exactamente uno de `domain` o `company_name` debe ser proporcionado (requisito XOR)

**Métodos del Builder:**
- `.first_name(name: &str)` - Establece el nombre (obligatorio)
- `.domain(domain: &str)` - Establece el nombre del dominio
- `.company_name(company: &str)` - Establece el nombre de la empresa
- `.middle_name(name: &str)` - Establece el segundo nombre (opcional)
- `.last_name(name: &str)` - Establece el apellido (opcional)
- `.call()` - Ejecuta la llamada API

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");

// Usando dominio
let result = zb.find_email_v2()
    .first_name("John")
    .domain("example.com")
    .last_name("Doe")
    .call()?;

// Usando nombre de empresa
let result = zb.find_email_v2()
    .first_name("John")
    .company_name("Example Inc")
    .last_name("Doe")
    .call()?;

// Con segundo nombre
let result = zb.find_email_v2()
    .first_name("John")
    .domain("example.com")
    .middle_name("Middle")
    .last_name("Doe")
    .call()?;
```

**Retorna:** `FindEmailResponseV2` que contiene:
- `email`: La dirección de correo encontrada
- `domain`: Nombre del dominio
- `confidence`: Nivel de confianza del correo (del campo `email_confidence`)
- `company_name`: Nombre de la empresa
- `did_you_mean`: Alternativa sugerida
- `failure_reason`: Razón del fallo si existe

### find_email (Deprecado)

⚠️ **Deprecado desde la versión 1.2.0** - Usa `find_email_v2` en su lugar.

Este método se mantiene por compatibilidad hacia atrás pero será eliminado en una versión futura. El nuevo método `find_email_v2` soporta tanto parámetros de dominio como de nombre de empresa.

**Ejemplo:**
```rust
let result = zb.find_email("example.com", "John", "", "Doe")?;
```

## Métodos de Búsqueda de Dominio

### domain_search_v2 (Recomendado)

Busca formatos de correo electrónico usando un dominio o nombre de empresa. Utiliza el patrón builder para llamadas API ergonómicas.

**Requisitos:**
- Exactamente uno de `domain` o `company_name` debe ser proporcionado (requisito XOR)

**Métodos del Builder:**
- `.domain(domain: &str)` - Establece el nombre del dominio
- `.company_name(company: &str)` - Establece el nombre de la empresa
- `.call()` - Ejecuta la llamada API

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");

// Usando dominio
let result = zb.domain_search_v2()
    .domain("example.com")
    .call()?;

// Usando nombre de empresa
let result = zb.domain_search_v2()
    .company_name("Example Inc")
    .call()?;
```

**Retorna:** `DomainSearchResponseV2` que contiene:
- `domain`: Nombre del dominio
- `company_name`: Nombre de la empresa
- `format`: Formato de correo encontrado
- `confidence`: Nivel de confianza
- `other_domain_formats`: Formatos alternativos con niveles de confianza
- `did_you_mean`: Alternativa sugerida
- `failure_reason`: Razón del fallo si existe

### domain_search (Deprecado)

⚠️ **Deprecado desde la versión 1.2.0** - Usa `domain_search_v2` en su lugar.

Este método se mantiene por compatibilidad hacia atrás pero será eliminado en una versión futura. El nuevo método `domain_search_v2` soporta tanto parámetros de dominio como de nombre de empresa.

**Ejemplo:**
```rust
let result = zb.domain_search("example.com")?;
```
