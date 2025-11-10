#### Cómo utilizar
Esta biblioteca utiliza la API de ZeroBounce, la cual requiere una clave de API. Consulta [esta guía](https://www.zerobounce.net/docs/api-dashboard#API_keys_management) para ver cómo obtener la tuya.

Revisa los [fragmentos de ejemplo](https://github.com/zerobounce/zerobounce-rust-api/tree/main/examples) para ver cómo puedes integrar esta biblioteca en tu propio proyecto.

## Opciones de Configuración

El cliente `ZeroBounce` puede ser configurado con diferentes URLs base dependiendo de tu región o requisitos.

### Configuración por Defecto

La forma más simple de crear un cliente es usando la URL de API por defecto:

```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
```

### Usando Valores de Enum

Puedes especificar una región usando el enum `ApiBaseUrl`:

```rust
use zero_bounce::{ZeroBounce, ApiBaseUrl};

// Región USA
let zb = ZeroBounce::with_base_url("tu_clave_api", ApiBaseUrl::USA);

// Región EU
let zb = ZeroBounce::with_base_url("tu_clave_api", ApiBaseUrl::EU);

// Por defecto (explícito)
let zb = ZeroBounce::with_base_url("tu_clave_api", ApiBaseUrl::Default);
```

### Usando URL Personalizada como String

También puedes proporcionar una URL base personalizada como string:

```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::with_base_url("tu_clave_api", "https://custom-api.example.com/v2/");
```

**URLs Base de API Disponibles:**
- `ApiBaseUrl::Default` - `https://api.zerobounce.net/v2/` (por defecto)
- `ApiBaseUrl::USA` - `https://api-us.zerobounce.net/v2/`
- `ApiBaseUrl::EU` - `https://api-eu.zerobounce.net/v2/`

Consulta el [ejemplo config_options](https://github.com/zerobounce/zerobounce-rust-api/tree/main/examples/config_options.rs) para una demostración completa de todas las opciones de configuración.

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

## Métodos de Cuenta y Uso

### get_credits

Obtiene el número de créditos restantes en tu cuenta de ZeroBounce.

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let credits = zb.get_credits()?;
println!("Créditos restantes: {}", credits);
```

**Retorna:** `i64` - El número de créditos restantes en tu cuenta

### get_api_usage

Obtiene estadísticas de uso de la API para un rango de fechas específico.

**Argumentos:**
- `start_date: NaiveDate` - Fecha de inicio del período de uso
- `end_date: NaiveDate` - Fecha de fin del período de uso

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;
use chrono::NaiveDate;

let zb = ZeroBounce::new("tu_clave_api");
let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
let usage = zb.get_api_usage(start, end)?;
println!("Total de llamadas API: {}", usage.total);
```

**Retorna:** `ApiUsage` que contiene:
- `total`: Número total de llamadas API en el período
- `status_valid`: Número de direcciones de correo válidas
- `status_invalid`: Número de direcciones de correo inválidas
- `status_catch_all`: Número de direcciones catch-all
- `status_do_not_mail`: Número de direcciones do-not-mail
- `status_spamtrap`: Número de direcciones spamtrap
- `status_abuse`: Número de direcciones abuse
- `status_unknown`: Número de direcciones desconocidas
- `sub_status_antispam_system`: Número de respuestas del sistema antispam
- `sub_status_greylisted`: Número de respuestas greylisted
- `sub_status_mail_server_temporary_error`: Número de errores temporales del servidor de correo
- `sub_status_forcible_disconnect`: Número de desconexiones forzadas
- `sub_status_mail_server_did_not_respond`: Número de servidores de correo que no respondieron
- `sub_status_timeout_exceeded`: Número de errores de timeout
- `sub_status_failed_smtp_connection`: Número de conexiones SMTP fallidas
- `sub_status_mailbox_quota_exceeded`: Número de errores de cuota de buzón excedida
- `sub_status_exception_occurred`: Número de excepciones
- `sub_status_possible_trap`: Número de posibles trampas
- `sub_status_role_based`: Número de direcciones basadas en roles
- `sub_status_global_suppression`: Número de direcciones suprimidas globalmente
- `sub_status_mailbox_not_found`: Número de errores de buzón no encontrado
- `sub_status_no_dns_entries`: Número de errores de sin entradas DNS
- `sub_status_failed_syntax_check`: Número de verificaciones de sintaxis fallidas
- `sub_status_possible_typo`: Número de posibles errores tipográficos
- `sub_status_unroutable_ip_address`: Número de direcciones IP no enrutables
- `sub_status_leading_period_removed`: Número de períodos iniciales removidos
- `sub_status_does_not_accept_mail`: Número de direcciones que no aceptan correo
- `sub_status_alias_address`: Número de direcciones alias
- `sub_status_role_based_catch_all`: Número de direcciones catch-all basadas en roles
- `sub_status_accept_all`: Número de direcciones accept-all
- `sub_status_disposable`: Número de direcciones desechables
- `sub_status_toxic`: Número de direcciones tóxicas

### get_api_usage_overall

Obtiene estadísticas generales de uso de la API desde el 1 de enero de 2000 hasta la fecha actual.

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let overall_usage = zb.get_api_usage_overall()?;
println!("Total de llamadas API en general: {}", overall_usage.total);
```

**Retorna:** `ApiUsage` - Misma estructura que `get_api_usage`

### get_activity_data

Obtiene datos de actividad para una dirección de correo electrónico específica, incluyendo cuándo fue vista por última vez enviando correos.

**Argumentos:**
- `email: &str` - La dirección de correo electrónico a verificar

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let activity = zb.get_activity_data("valid@example.com")?;
println!("Encontrado: {}", activity.found);
if activity.found {
    println!("Estado activo: {}", activity.active_status);
}
```

**Retorna:** `ActivityData` que contiene:
- `found`: Si se encontraron datos de actividad para el correo
- `active_status`: Estado activo de la dirección de correo
- `active_date`: Fecha en que el correo estuvo activo por última vez

## Métodos de Validación de Correo

### validate_email

Valida una sola dirección de correo electrónico.

**Argumentos:**
- `email: &str` - La dirección de correo electrónico a validar

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let validation = zb.validate_email("valid@example.com")?;
println!("Estado: {}", validation.status);
println!("Sub estado: {:?}", validation.sub_status);
```

**Retorna:** `ZBValidation` que contiene:
- `address`: La dirección de correo que fue validada
- `status`: Estado de validación (`valid`, `invalid`, `catch-all`, `unknown`, `spamtrap`, `abuse`, `do_not_mail`)
- `sub_status`: Sub-estado que proporciona más detalles sobre el resultado de la validación
- `account`: Nombre de la cuenta (si está disponible)
- `domain`: Nombre del dominio
- `did_you_mean`: Dirección de correo alternativa sugerida
- `domain_age_days`: Edad del dominio en días
- `smtp_provider`: Nombre del proveedor SMTP
- `mx_found`: Si se encontraron registros MX
- `mx_record`: Valor del registro MX
- `firstname`: Nombre asociado con el correo (si está disponible)
- `lastname`: Apellido asociado con el correo (si está disponible)
- `gender`: Género asociado con el correo (si está disponible)
- `country`: País asociado con el correo (si está disponible)
- `region`: Región asociada con el correo (si está disponible)
- `city`: Ciudad asociada con el correo (si está disponible)
- `zipcode`: Código postal asociado con el correo (si está disponible)
- `processed_at`: Marca de tiempo cuando se procesó la validación

### validate_email_and_ip

Valida una dirección de correo electrónico con una dirección IP opcional para una validación más precisa.

**Argumentos:**
- `email: &str` - La dirección de correo electrónico a validar
- `ip_address: &str` - Dirección IP opcional (puede ser cadena vacía)

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let validation = zb.validate_email_and_ip("valid@example.com", "99.110.204.1")?;
println!("Estado: {}", validation.status);
```

**Retorna:** `ZBValidation` - Misma estructura que `validate_email`

### batch_validate

Valida múltiples direcciones de correo electrónico en una sola llamada API. Más eficiente que validar correos uno por uno.

**Argumentos:**
- `emails_and_ip_addresses: Vec<(String, String)>` - Vector de tuplas que contienen pares (email, ip_address). La dirección IP puede ser cadena vacía.

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let emails_and_ips = vec![
    ("valid@example.com".to_string(), "99.110.204.1".to_string()),
    ("invalid@example.com".to_string(), "".to_string()),
];
let batch_result = zb.batch_validate(emails_and_ips)?;
println!("Lote de correos: {:#?}", batch_result.email_batch);
```

**Retorna:** `ZBBatchValidation` que contiene:
- `email_batch`: Vector de resultados `ZBValidation` para cada dirección de correo

## Métodos de Validación Masiva

La validación masiva te permite subir un archivo que contiene múltiples direcciones de correo electrónico para validación. El proceso implica enviar un archivo, verificar su estado, obtener resultados y opcionalmente eliminar el archivo.

### bulk_validation_file_submit

Envía un archivo para validación masiva de correos electrónicos.

**Argumentos:**
- `zb_file: &ZBFile` - Una instancia `ZBFile` que contiene las direcciones de correo a validar

**Ejemplo:**
```rust
use zero_bounce::{ZeroBounce, ZBFile};

let zb = ZeroBounce::new("tu_clave_api");
let file_content = vec![
    "email1@example.com".to_string(),
    "email2@example.com".to_string(),
];
let zb_file = ZBFile::from_content(file_content)
    .set_has_header_row(false)
    .set_remove_duplicate(true);
let submit_result = zb.bulk_validation_file_submit(&zb_file)?;
println!("ID de archivo: {:?}", submit_result.file_id);
```

**Retorna:** `ZBFileFeedback` que contiene:
- `success`: Si el envío del archivo fue exitoso
- `message`: Mensaje de estado
- `file_id`: ID de archivo opcional para usar en verificaciones de estado y obtención de resultados

### bulk_validation_file_status_check

Verifica el estado de procesamiento de un archivo de validación masiva enviado.

**Argumentos:**
- `file_id: &str` - El ID de archivo devuelto de `bulk_validation_file_submit`

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let status = zb.bulk_validation_file_status_check("file_id_here")?;
println!("Porcentaje completo: {}%", status.complete_percentage);
println!("Conteo de éxito: {}", status.success_count);
```

**Retorna:** `ZBFileStatus` que contiene:
- `success`: Si la verificación de estado fue exitosa
- `message`: Mensaje de estado
- `file_id`: El ID de archivo
- `file_name`: Nombre del archivo
- `upload_date`: Fecha en que se subió el archivo
- `file_status`: Estado actual del archivo
- `complete_percentage`: Porcentaje de procesamiento completado
- `return_url`: URL para obtener resultados
- `success_count`: Número de correos procesados exitosamente
- `error_count`: Número de errores encontrados

### bulk_validation_result_fetch

Obtiene los resultados de un archivo de validación masiva completado.

**Argumentos:**
- `file_id: &str` - El ID de archivo devuelto de `bulk_validation_file_submit`

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;
use zero_bounce::utility::structures::bulk::ZBBulkResponse;

let zb = ZeroBounce::new("tu_clave_api");
let result = zb.bulk_validation_result_fetch("file_id_here")?;
match result {
    ZBBulkResponse::Content(bytes) => {
        // Contenido del archivo (formato CSV)
        println!("Recibidos {} bytes", bytes.len());
    },
    ZBBulkResponse::Feedback(feedback) => {
        // Error o retroalimentación de estado
        println!("Mensaje: {}", feedback.message);
    },
}
```

**Retorna:** `ZBBulkResponse` que puede ser:
- `ZBBulkResponse::Content(Vec<u8>)` - El contenido del archivo como bytes (típicamente formato CSV)
- `ZBBulkResponse::Feedback(ZBFileFeedback)` - Error o retroalimentación de estado si el archivo no está listo o ocurrió un error

### bulk_validation_result_delete

Elimina un archivo de resultados de validación masiva de los servidores de ZeroBounce.

**Argumentos:**
- `file_id: &str` - El ID de archivo devuelto de `bulk_validation_file_submit`

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let delete_result = zb.bulk_validation_result_delete("file_id_here")?;
println!("Eliminación exitosa: {}", delete_result.success);
```

**Retorna:** `ZBFileFeedback` que contiene:
- `success`: Si la eliminación fue exitosa
- `message`: Mensaje de estado

## Métodos de Puntuación IA

La Puntuación IA te permite subir un archivo que contiene direcciones de correo electrónico para obtener puntuaciones de calidad impulsadas por IA. El proceso es similar a la validación masiva: enviar un archivo, verificar estado, obtener resultados y opcionalmente eliminar el archivo.

### ai_scoring_file_submit

Envía un archivo para análisis de puntuación IA.

**Argumentos:**
- `zb_file: &ZBFile` - Una instancia `ZBFile` que contiene las direcciones de correo a puntuar

**Ejemplo:**
```rust
use zero_bounce::{ZeroBounce, ZBFile};

let zb = ZeroBounce::new("tu_clave_api");
let file_content = vec![
    "email1@example.com".to_string(),
    "email2@example.com".to_string(),
];
let zb_file = ZBFile::from_content(file_content)
    .set_has_header_row(false)
    .set_remove_duplicate(true);
let submit_result = zb.ai_scoring_file_submit(&zb_file)?;
println!("ID de archivo: {:?}", submit_result.file_id);
```

**Retorna:** `ZBFileFeedback` - Misma estructura que `bulk_validation_file_submit`

### ai_scoring_file_status_check

Verifica el estado de procesamiento de un archivo de puntuación IA enviado.

**Argumentos:**
- `file_id: &str` - El ID de archivo devuelto de `ai_scoring_file_submit`

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let status = zb.ai_scoring_file_status_check("file_id_here")?;
println!("Porcentaje completo: {}%", status.complete_percentage);
```

**Retorna:** `ZBFileStatus` - Misma estructura que `bulk_validation_file_status_check`

### ai_scoring_result_fetch

Obtiene los resultados de un archivo de puntuación IA completado.

**Argumentos:**
- `file_id: &str` - El ID de archivo devuelto de `ai_scoring_file_submit`

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;
use zero_bounce::utility::structures::bulk::ZBBulkResponse;

let zb = ZeroBounce::new("tu_clave_api");
let result = zb.ai_scoring_result_fetch("file_id_here")?;
match result {
    ZBBulkResponse::Content(bytes) => {
        println!("Recibidos {} bytes", bytes.len());
    },
    ZBBulkResponse::Feedback(feedback) => {
        println!("Mensaje: {}", feedback.message);
    },
}
```

**Retorna:** `ZBBulkResponse` - Misma estructura que `bulk_validation_result_fetch`

### ai_scoring_result_delete

Elimina un archivo de resultados de puntuación IA de los servidores de ZeroBounce.

**Argumentos:**
- `file_id: &str` - El ID de archivo devuelto de `ai_scoring_file_submit`

**Ejemplo:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("tu_clave_api");
let delete_result = zb.ai_scoring_result_delete("file_id_here")?;
println!("Eliminación exitosa: {}", delete_result.success);
```

**Retorna:** `ZBFileFeedback` - Misma estructura que `bulk_validation_result_delete`
