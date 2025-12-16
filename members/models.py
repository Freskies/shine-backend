from django.db import models
from django.utils import timezone


class Member(models.Model):
    name = models.CharField(max_length=100)
    birth = models.DateField()
    phone = models.CharField(max_length=15)
    mail = models.EmailField()
    fiscalCode = models.CharField(max_length=16, unique=True)
    modifiers = models.TextField(blank=True, null=True)

    @property
    def latest_medical_certificate(self):
        return self.medical_certificates.order_by('-issue_date').first()

    @property
    def is_medical_certificate_valid(self):
        latest_certificate = self.latest_medical_certificate
        return latest_certificate and latest_certificate.expiration_date >= timezone.now().date()

class MedicalCertificate(models.Model):
    member = models.ForeignKey(Member, on_delete=models.CASCADE, related_name='medical_certificates')
    issue_date = models.DateField()
    expiration_date = models.DateField()
    file = models.FileField(upload_to='medical_certificates/')
    notes = models.TextField(blank=True, null=True)