# A small example that needs no cloud account.
terraform {
  required_providers {
    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }
  }
}

variable "length" {
  type        = number
  default     = 2
  description = "Number of words in the pet name."
}

resource "random_pet" "name" {
  length = var.length
}

resource "random_id" "suffix" {
  byte_length = 4
  keepers = {
    pet = random_pet.name.id
  }
}

output "pet" {
  value = "${random_pet.name.id}-${random_id.suffix.hex}"
}
