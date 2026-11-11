
window.onload = function() {
  // Build a system
  var url = window.location.search.match(/url=([^&]+)/);
  if (url && url.length > 1) {
    url = decodeURIComponent(url[1]);
  } else {
    url = window.location.origin;
  }
  var options = {
  "swaggerDoc": {
    "openapi": "3.0.3",
    "info": {
      "title": "CPDV Bible API",
      "description": "A production-ready REST API serving the Catholic Public Domain Version (CPDV) of the Bible. 73 books, 1,335 chapters, 35,837 verses. All data served from memory for sub-millisecond responses.",
      "version": "1.0.0",
      "license": {
        "name": "Public Domain"
      }
    },
    "servers": [
      {
        "url": "/api/v1",
        "description": "API v1"
      }
    ],
    "components": {
      "securitySchemes": {
        "ApiKeyAuth": {
          "type": "apiKey",
          "in": "header",
          "name": "X-API-Key",
          "description": "API key for authenticating requests. Public endpoints (/health, /docs) do not require a key."
        }
      }
    },
    "security": [
      {
        "ApiKeyAuth": []
      }
    ],
    "tags": [
      {
        "name": "Books",
        "description": "Browse books, chapters, and verses"
      },
      {
        "name": "Search",
        "description": "Full-text search and discovery"
      },
      {
        "name": "Metadata",
        "description": "Bible statistics and groupings"
      },
      {
        "name": "Health",
        "description": "Service health"
      }
    ],
    "paths": {
      "/books": {
        "get": {
          "tags": [
            "Books"
          ],
          "summary": "List all books",
          "description": "Returns metadata for all 73 books in the CPDV Bible.",
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "array",
                        "items": {
                          "type": "object",
                          "properties": {
                            "name": {
                              "type": "string",
                              "example": "Genesis"
                            },
                            "key": {
                              "type": "string",
                              "example": "Genesis"
                            },
                            "testament": {
                              "type": "string",
                              "enum": [
                                "OT",
                                "NT"
                              ],
                              "example": "OT"
                            },
                            "chapterCount": {
                              "type": "integer",
                              "example": 50
                            },
                            "verseCount": {
                              "type": "integer",
                              "example": 1530
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/books/{book}": {
        "get": {
          "tags": [
            "Books"
          ],
          "summary": "Get book metadata",
          "description": "Returns metadata for a single book. Accepts case-insensitive names and common aliases (e.g. \"gen\", \"1samuel\", \"1-samuel\", \"sos\", \"rev\").",
          "parameters": [
            {
              "name": "book",
              "in": "path",
              "required": true,
              "schema": {
                "type": "string"
              },
              "example": "Genesis"
            }
          ],
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "object",
                        "properties": {
                          "name": {
                            "type": "string",
                            "example": "Genesis"
                          },
                          "key": {
                            "type": "string",
                            "example": "Genesis"
                          },
                          "testament": {
                            "type": "string",
                            "enum": [
                              "OT",
                              "NT"
                            ],
                            "example": "OT"
                          },
                          "chapterCount": {
                            "type": "integer",
                            "example": 50
                          },
                          "verseCount": {
                            "type": "integer",
                            "example": 1530
                          }
                        }
                      }
                    }
                  }
                }
              }
            },
            "404": {
              "description": "Not found",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "error"
                      },
                      "error": {
                        "type": "string",
                        "example": "Book not found: xyz"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/books/{book}/chapters": {
        "get": {
          "tags": [
            "Books"
          ],
          "summary": "List chapters in a book",
          "description": "Returns chapter numbers and verse counts for the specified book.",
          "parameters": [
            {
              "name": "book",
              "in": "path",
              "required": true,
              "schema": {
                "type": "string"
              },
              "example": "Genesis"
            }
          ],
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "array",
                        "items": {
                          "type": "object",
                          "properties": {
                            "chapter": {
                              "type": "integer",
                              "example": 1
                            },
                            "verseCount": {
                              "type": "integer",
                              "example": 31
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            },
            "404": {
              "description": "Not found",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "error"
                      },
                      "error": {
                        "type": "string",
                        "example": "Book not found: xyz"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/books/{book}/{chapter}": {
        "get": {
          "tags": [
            "Books"
          ],
          "summary": "Get all verses in a chapter",
          "description": "Returns all verses for the specified book and chapter.",
          "parameters": [
            {
              "name": "book",
              "in": "path",
              "required": true,
              "schema": {
                "type": "string"
              },
              "example": "Genesis"
            },
            {
              "name": "chapter",
              "in": "path",
              "required": true,
              "schema": {
                "type": "integer"
              },
              "example": 1
            }
          ],
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "array",
                        "items": {
                          "type": "object",
                          "properties": {
                            "book": {
                              "type": "string",
                              "example": "Genesis"
                            },
                            "chapter": {
                              "type": "integer",
                              "example": 1
                            },
                            "verse": {
                              "type": "integer",
                              "example": 1
                            },
                            "text": {
                              "type": "string",
                              "example": "In the beginning, God created heaven and earth"
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            },
            "404": {
              "description": "Not found",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "error"
                      },
                      "error": {
                        "type": "string",
                        "example": "Chapter not found"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/books/{book}/{chapter}/{verse}": {
        "get": {
          "tags": [
            "Books"
          ],
          "summary": "Get a verse or verse range",
          "description": "Returns a single verse or a range (e.g. \"1-5\"). Use format `startVerse-endVerse` for ranges.",
          "parameters": [
            {
              "name": "book",
              "in": "path",
              "required": true,
              "schema": {
                "type": "string"
              },
              "example": "Genesis"
            },
            {
              "name": "chapter",
              "in": "path",
              "required": true,
              "schema": {
                "type": "integer"
              },
              "example": 1
            },
            {
              "name": "verse",
              "in": "path",
              "required": true,
              "schema": {
                "type": "string"
              },
              "description": "Single verse number or range (e.g. \"1\" or \"1-5\")",
              "example": "1"
            }
          ],
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "object",
                        "properties": {
                          "book": {
                            "type": "string",
                            "example": "Genesis"
                          },
                          "chapter": {
                            "type": "integer",
                            "example": 1
                          },
                          "verse": {
                            "type": "integer",
                            "example": 1
                          },
                          "text": {
                            "type": "string",
                            "example": "In the beginning, God created heaven and earth"
                          }
                        }
                      }
                    }
                  }
                }
              }
            },
            "404": {
              "description": "Not found",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "error"
                      },
                      "error": {
                        "type": "string",
                        "example": "Verse not found"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/search": {
        "get": {
          "tags": [
            "Search"
          ],
          "summary": "Full-text search",
          "description": "Search verse text with optional book and testament filters. Returns paginated results.",
          "parameters": [
            {
              "name": "q",
              "in": "query",
              "required": true,
              "schema": {
                "type": "string"
              },
              "description": "Search query",
              "example": "faith hope love"
            },
            {
              "name": "book",
              "in": "query",
              "required": false,
              "schema": {
                "type": "string"
              },
              "description": "Filter by book name"
            },
            {
              "name": "testament",
              "in": "query",
              "required": false,
              "schema": {
                "type": "string",
                "enum": [
                  "OT",
                  "NT"
                ]
              },
              "description": "Filter by testament"
            },
            {
              "name": "limit",
              "in": "query",
              "required": false,
              "schema": {
                "type": "integer",
                "default": 20,
                "minimum": 1,
                "maximum": 100
              }
            },
            {
              "name": "offset",
              "in": "query",
              "required": false,
              "schema": {
                "type": "integer",
                "default": 0,
                "minimum": 0
              }
            }
          ],
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "array",
                        "items": {
                          "type": "object",
                          "properties": {
                            "book": {
                              "type": "string",
                              "example": "Genesis"
                            },
                            "chapter": {
                              "type": "integer",
                              "example": 1
                            },
                            "verse": {
                              "type": "integer",
                              "example": 1
                            },
                            "text": {
                              "type": "string"
                            },
                            "snippet": {
                              "type": "string"
                            }
                          }
                        }
                      },
                      "meta": {
                        "type": "object",
                        "properties": {
                          "total": {
                            "type": "integer",
                            "example": 42
                          },
                          "limit": {
                            "type": "integer",
                            "example": 20
                          },
                          "offset": {
                            "type": "integer",
                            "example": 0
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/random": {
        "get": {
          "tags": [
            "Search"
          ],
          "summary": "Random verse",
          "description": "Returns a random verse from the entire Bible.",
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "object",
                        "properties": {
                          "book": {
                            "type": "string",
                            "example": "Genesis"
                          },
                          "chapter": {
                            "type": "integer",
                            "example": 1
                          },
                          "verse": {
                            "type": "integer",
                            "example": 1
                          },
                          "text": {
                            "type": "string",
                            "example": "In the beginning, God created heaven and earth"
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/verse-of-the-day": {
        "get": {
          "tags": [
            "Search"
          ],
          "summary": "Verse of the day",
          "description": "Returns a deterministic verse based on the current date. Same verse for all requests on the same day.",
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "object",
                        "properties": {
                          "book": {
                            "type": "string",
                            "example": "Genesis"
                          },
                          "chapter": {
                            "type": "integer",
                            "example": 1
                          },
                          "verse": {
                            "type": "integer",
                            "example": 1
                          },
                          "text": {
                            "type": "string",
                            "example": "In the beginning, God created heaven and earth"
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/testaments": {
        "get": {
          "tags": [
            "Metadata"
          ],
          "summary": "Testament groupings",
          "description": "Returns Old and New Testament book lists.",
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "object",
                        "properties": {
                          "OT": {
                            "type": "array",
                            "items": {
                              "type": "string"
                            }
                          },
                          "NT": {
                            "type": "array",
                            "items": {
                              "type": "string"
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/stats": {
        "get": {
          "tags": [
            "Metadata"
          ],
          "summary": "Bible statistics",
          "description": "Returns total counts for books, chapters, and verses.",
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "object",
                        "properties": {
                          "books": {
                            "type": "integer",
                            "example": 73
                          },
                          "chapters": {
                            "type": "integer",
                            "example": 1335
                          },
                          "verses": {
                            "type": "integer",
                            "example": 35837
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      },
      "/health": {
        "get": {
          "tags": [
            "Health"
          ],
          "summary": "Health check",
          "description": "Returns service health status and uptime.",
          "responses": {
            "200": {
              "description": "Successful response",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "status": {
                        "type": "string",
                        "example": "ok"
                      },
                      "data": {
                        "type": "object",
                        "properties": {
                          "status": {
                            "type": "string",
                            "example": "healthy"
                          },
                          "uptime": {
                            "type": "number",
                            "example": 123.456
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  },
  "customOptions": {}
};
  url = options.swaggerUrl || url
  var urls = options.swaggerUrls
  var customOptions = options.customOptions
  var spec1 = options.swaggerDoc
  var swaggerOptions = {
    spec: spec1,
    url: url,
    urls: urls,
    dom_id: '#swagger-ui',
    deepLinking: true,
    presets: [
      SwaggerUIBundle.presets.apis,
      SwaggerUIStandalonePreset
    ],
    plugins: [
      SwaggerUIBundle.plugins.DownloadUrl
    ],
    layout: "StandaloneLayout"
  }
  for (var attrname in customOptions) {
    swaggerOptions[attrname] = customOptions[attrname];
  }
  var ui = SwaggerUIBundle(swaggerOptions)

  if (customOptions.oauth) {
    ui.initOAuth(customOptions.oauth)
  }

  if (customOptions.preauthorizeApiKey) {
    const key = customOptions.preauthorizeApiKey.authDefinitionKey;
    const value = customOptions.preauthorizeApiKey.apiKeyValue;
    if (!!key && !!value) {
      const pid = setInterval(() => {
        const authorized = ui.preauthorizeApiKey(key, value);
        if(!!authorized) clearInterval(pid);
      }, 500)

    }
  }

  if (customOptions.authAction) {
    ui.authActions.authorize(customOptions.authAction)
  }

  window.ui = ui
}
